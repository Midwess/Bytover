use std::collections::HashMap;
use std::mem;
use std::ops::Deref;
use std::sync::{Arc, Weak};
use std::time::Duration;

use core_services::local_storage::file_system::File;
use schema::devlog::bitbridge::peer_message_body::{Request, Response};
use schema::devlog::bitbridge::{
    resource_thumbnail_message,
    CancelTransferSessionRequest,
    IntroduceRequestMessage,
    IntroduceResponseMessage,
    PeerErrorsMessage,
    PeerMessageBody,
    ResourceThumbnailMessage,
    TransferRequestMessage,
    TransferResponseMessage,
    TransferSessionMessage,
    VoidResponseMessage
};
use thiserror::Error;
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::{broadcast, Mutex, OnceCell};
use tokio::time::timeout;
use tokio::{select, spawn};
use webrtc::data_channel::data_channel_state::RTCDataChannelState;

use crate::app::file_system::file::LocalResourcePath;
use crate::app::file_system::workdir::WorkDir;
use crate::app::operations::p2p::P2POperationOutput;
use crate::app::operations::transfer::TransferOperationOutput;
use crate::app::operations::CoreOperationOutput;
use crate::app::transfer::session::{TransferSession, TransferSessionStatus};
use crate::entities::peer::Peer as PeerEntity;
use crate::native::message_to_shell::MessageToShell;
use crate::network::webrtc::message_channel::PeerRequest;
use crate::{serialize, ShellRuntime};

use super::connection::{ConnectionWebRtc, ConnectionWebRtcErrors};
use super::data_channel::{DataChannel, DataChannelError};
use super::throughput::ThroughputController;

#[derive(Debug, Error)]
pub enum PeerErrors {
    #[error("Failed to connect to peer {:?}", .0)]
    ConnectionError(#[from] ConnectionWebRtcErrors),
    #[error("Peer response error {:?}", .0)]
    ResponseError(#[from] PeerErrorsMessage),
    #[error("No response from peer")]
    NoResponseFromPeer,
    #[error("Failed to send session")]
    FailedToSendSession(String),
    #[error("Error while receiving resource")]
    ErrorWhileReceivingResource(String),
    #[error("Error while sending resource")]
    ErrorWhileSendingResource(String),
    #[error("Channel error {:?}", .0)]
    ChannelError(#[from] DataChannelError),
    #[error("Failed to send resource thumbnail")]
    FailedToSendResourceThumbnail(String)
}

// A higher level that utilize the WebRtc connection
// To develop a transferable peer-to-peer logic
pub struct PeerCommunication {
    mine: PeerEntity,
    pub peer: PeerEntity,
    connection: Arc<ConnectionWebRtc>,
    shell_runtime: Arc<dyn ShellRuntime>,
    data_channel_tx: broadcast::Sender<Arc<DataChannel>>,
    peer_event_request_id: OnceCell<u32>,
    throughput_controller: Arc<ThroughputController>,
    active_sessions: Arc<Mutex<HashMap<u64, Weak<Mutex<TransferSession>>>>>,
    work_dir: WorkDir
}

impl PeerCommunication {
    pub async fn upgrade(
        work_dir: WorkDir,
        connection: ConnectionWebRtc,
        current_peer: PeerEntity,
        peer_id: u128,
        shell_runtime: Arc<dyn ShellRuntime>,
        throughput_controller: Arc<ThroughputController>
    ) -> Result<Arc<Self>, PeerErrors> {
        let connection = Arc::new(connection);
        let peer = if current_peer.id() < peer_id {
            let introduce_request = IntroduceRequestMessage {
                mine: current_peer.clone().into()
            };

            let response = connection
                .send::<IntroduceResponseMessage>(Request::IntroduceRequest(introduce_request), None, None)
                .await??;
            response.peer.into()
        } else {
            let mut peer_result = None;
            while let Ok(request) = connection.next_request(None).await {
                if let Request::IntroduceRequest(introduction) = request.message() {
                    let peer: PeerEntity = introduction.mine.clone().into();
                    request
                        .resolve(Response::IntroduceResponse(IntroduceResponseMessage {
                            peer: current_peer.clone().into()
                        }))
                        .await?;
                    peer_result = Some(peer);
                    break;
                }
            }

            peer_result.ok_or(PeerErrors::NoResponseFromPeer)?
        };

        log::info!(target: "peer", "Connected to peer {:?}, size = {}", peer, mem::size_of::<PeerCommunication>());

        // Indicate that the maximum number of concurrent resources per peer is 16
        let (data_channel_tx, _) = broadcast::channel(16);

        let me = Arc::new(Self {
            peer_event_request_id: OnceCell::new(),
            mine: current_peer,
            peer,
            connection,
            shell_runtime: shell_runtime.clone(),
            data_channel_tx,
            throughput_controller,
            active_sessions: Arc::new(Mutex::new(HashMap::new())),
            work_dir
        });

        me.handle_data_channel();

        Ok(me)
    }

    pub async fn next_peers_event(&self, core_request_id: u32) -> Result<(), PeerErrors> {
        let _ = self.peer_event_request_id.set(core_request_id);

        select! {
            request = self.connection.next_request(None) => {
                let request = request?;
                match request.take_message() {
                    Request::TransferRequest(transfer_request) => {
                        let response = CoreOperationOutput::P2P(P2POperationOutput::ReceivedSessionRequest {
                            request_id: request.id.clone(),
                            remote_session: transfer_request.session.clone()
                        });
                        self.shell_runtime.clone()
                            .msg_from_native_bg(serialize(&MessageToShell::HandleResponse(core_request_id, response)));
                    }
                    Request::CancelRequest(cancel_request) => {
                        log::info!(target: "peer", "Received cancel request from peer {:?}", self.peer.id());
                        let response = CoreOperationOutput::P2P(P2POperationOutput::CancelSessionRequest {
                            request_id: request.id.clone(),
                            session_id: cancel_request.session_id as u64
                        });

                        self.shell_runtime.clone()
                            .msg_from_native_bg(serialize(&MessageToShell::HandleResponse(core_request_id, response)));
                    }
                    Request::ResourceThumbnailFullfill(mut thumbnail_message) => {
                        log::info!(target: "peer", "Received thumbnail for resource {:?}", thumbnail_message.resource_id);
                        if let Some(resource_thumbnail_message::Data::Png(data)) = thumbnail_message.data.take() {
                            let resource_id = thumbnail_message.resource_id;
                            let current_index = thumbnail_message.current_index;
                            let data_length = thumbnail_message.data_length;
                            let shell_runtime = self.shell_runtime.clone();
                            let thumbnail_path = self.work_dir.thumbnails(format!("{resource_id}.png"));
                            let session_id = thumbnail_message.session_id as u64;
                            let workdir = self.work_dir.clone();

                            tokio::spawn(async move {
                                let saved_path = thumbnail_path;

                                // Check if this is the first chunk
                                if current_index == 0 {
                                    // Delete existing file if it exists
                                    if let Ok(existing) = File::existing(saved_path.clone()).await {
                                        log::info!(target: "peer", "Deleting existing thumbnail file");
                                        let _ = existing.delete().await;
                                    }

                                    // Create new file
                                    if let Ok(mut new_file) = File::new(None, saved_path.clone()).await {
                                        let _ = new_file.write(data.clone()).await;
                                    } else {
                                        log::error!(target: "peer", "Failed to create new thumbnail file");
                                    }
                                }
                                else {
                                    // Append to existing file
                                    if let Ok(mut existing_file) = File::existing(saved_path.clone()).await {
                                        let _ = existing_file.write(data.clone()).await;
                                    }
                                    else {
                                        log::error!(target: "peer", "Thumbnail file not found for appending");
                                    }
                                }

                                // Check if this is the last chunk
                                let chunk_size = data.len() as i64;
                                if current_index + chunk_size >= data_length - 1 {
                                    let msg = CoreOperationOutput::P2P(P2POperationOutput::ThumbnailFullfillment {
                                        session_id,
                                        resource_id: resource_id as u64,
                                        path: workdir.to_relative_path(&LocalResourcePath::AbsolutePath(saved_path))
                                    });

                                    let _ = shell_runtime
                                        .msg_from_native(serialize(&MessageToShell::HandleResponse(core_request_id, msg)))
                                        .await;
                                }

                                let _ = request.resolve(Response::VoidResponse(VoidResponseMessage {})).await;
                            });
                        }
                    }
                    _ => {}
                }
            }
        };

        Ok(())
    }

    fn handle_data_channel(&self) {
        self.connection.peer_connection.on_data_channel({
            let data_channel_tx = self.data_channel_tx.clone();
            let shell_runtime = self.shell_runtime.clone();
            let throughput_controller = self.throughput_controller.clone();
            let active_sessions = self.active_sessions.clone();
            Box::new(move |d: Arc<webrtc::data_channel::RTCDataChannel>| {
                let active_sessions = active_sessions.clone();
                let data_channel_tx = data_channel_tx.clone();
                let shell_runtime = shell_runtime.clone();
                let throughput_controller = throughput_controller.clone();
                Box::pin(async move {
                    let active_sessions = active_sessions.lock().await;
                    let Ok((resource_id, session_id)) = DataChannel::from_label(d.label()) else {
                        log::warn!(target: "peer", "Failed to parse data channel label");
                        let _ = d.close().await;
                        return;
                    };

                    let Some(session) = active_sessions.get(&session_id).and_then(|s| s.upgrade()) else {
                        log::warn!(target: "peer", "Session not found");
                        let _ = d.close().await;
                        return;
                    };

                    let mut session_guard = session.lock().await;

                    if session_guard.is_completed() {
                        log::warn!(target: "peer", "Session is completed");
                        let _ = d.close().await;
                        return;
                    }

                    let resource_progress = session_guard.resource_mut_progress(resource_id).expect("Progress not found");
                    if resource_progress.is_completed() {
                        log::warn!(target: "peer", "Resource is already completed");
                        let _ = d.close().await;
                        return;
                    }

                    drop(session_guard);
                    let data_channel = match DataChannel::from_channel(
                        d,
                        shell_runtime.clone(),
                        throughput_controller.clone(),
                        Arc::downgrade(&session)
                    )
                    .await
                    {
                        Ok(data_channel) => {
                            // If channel got lagged, it will be closed automatically
                            data_channel.auto_close(true);
                            Arc::new(data_channel)
                        }
                        Err(e) => {
                            log::error!(target: "peer", "Failed to create data channel {:?}", e);
                            return;
                        }
                    };

                    if let Err(e) = data_channel_tx.send(data_channel) {
                        log::error!(target: "peer", "Failed to broadcast data channel {:?}", e.to_string());
                    }
                })
            })
        });
    }

    pub async fn send_session(&self, session: TransferSession, core_request_id: u32) -> Result<TransferSessionStatus, PeerErrors> {
        let order_id = session.order_id;

        let all_thumbnails_and_resource_ids = session
            .resources
            .iter()
            .filter_map(|r| r.thumbnail_path.as_ref().map(|path| (r.order_id, path)))
            .filter_map(|(resource_id, path)| path.disk_path().map(|path| (resource_id, path)))
            .collect::<Vec<_>>();

        let session = Arc::new(Mutex::new(session));
        let weak_session = Arc::downgrade(&session);
        self.active_sessions.lock().await.insert(order_id, weak_session.clone());
        let session_metadata_task = {
            let session_ref = weak_session.clone();
            async move {
                if let Some(session_ref) = session_ref.upgrade() {
                    let mut session_guard = session_ref.lock().await;
                    let transfer_session = TransferSessionMessage {
                        order_id: session_guard.order_id,
                        resources: session_guard.resources.iter().map(|r| r.to_proto()).collect()
                    };

                    log::info!(target: "peer", "Sending session to peer {:?}", self.peer.id());
                    let request = Request::TransferRequest(TransferRequestMessage {
                        session: transfer_session.clone()
                    });

                    match self.connection.send::<TransferResponseMessage>(request, None, None).await {
                        Ok(Ok(_)) => {}
                        Ok(Err(e)) => {
                            log::error!(target: "peer", "Failed to send session to peer {:?}", e);
                            session_guard.force_complete(e.to_string());
                            return Err(PeerErrors::FailedToSendSession(e.to_string()));
                        }
                        Err(e) => {
                            log::error!(target: "peer", "Failed to send session to peer {:?}", e);
                            session_guard.force_complete(e.to_string());
                            return Err(PeerErrors::FailedToSendSession(e.to_string()));
                        }
                    }
                }

                for (resource_id, path) in all_thumbnails_and_resource_ids {
                    let Some(session_ref) = session_ref.upgrade() else {
                        log::warn!(target: "peer", "Session is already closed");
                        return Ok(());
                    };

                    if session_ref.lock().await.is_completed() {
                        log::warn!(target: "peer", "Session is already completed");
                        return Ok(());
                    }

                    if let Err(e) = self.send_resource_thumbnail(order_id, resource_id, path.as_str()).await {
                        log::error!(target: "peer", "Failed to send resource thumbnail {:?}", e);
                    } else {
                        log::info!(target: "peer", "Sent resource thumbnail {:?}", resource_id);
                    }
                }

                Ok(())
            }
        };

        let transfer_resources_task = {
            let session = session.clone();
            let active_sessions = self.active_sessions.clone();
            async move {
                let result = self.send_session_resources(session, core_request_id).await;
                active_sessions.lock().await.remove(&order_id);
                result
            }
        };

        let (request_result, transfer_result) = tokio::join!(session_metadata_task, transfer_resources_task);

        request_result.and(transfer_result)
    }

    pub async fn send_session_resources(
        &self,
        session: Arc<Mutex<TransferSession>>,
        core_request_id: u32
    ) -> Result<TransferSessionStatus, PeerErrors> {
        let mut channel_subscription = self.data_channel_tx.subscribe();

        loop {
            let session_guard = session.lock().await;
            if session_guard.is_completed() {
                return Ok(session_guard.status());
            }

            drop(session_guard);

            let data_channel = match timeout(Duration::from_secs(15), channel_subscription.recv()).await {
                Ok(Ok(data_channel)) => {
                    // Make sure the channel is not closed automatically
                    // while the buffer is not empty
                    data_channel.auto_close(false);
                    data_channel
                }
                Err(e) => {
                    session.lock().await.force_complete("No response from peer within timeout".to_string());
                    return Err(PeerErrors::NoResponseFromPeer);
                }
                Ok(Err(e)) => match e {
                    RecvError::Closed => {
                        session.lock().await.force_complete("No response from peer, channel closed".to_string());
                        return Err(PeerErrors::NoResponseFromPeer);
                    }
                    RecvError::Lagged(e) => {
                        log::warn!(target: "peer", "Data channel lagged by {:?} channels", e);
                        continue;
                    }
                }
            };

            if data_channel.ready_state() == RTCDataChannelState::Closed || data_channel.ready_state() == RTCDataChannelState::Closing
            {
                log::warn!(target: "peer", "Data channel is closed or closing, skipping...");
                continue;
            }

            let shell_runtime = self.shell_runtime.clone();

            let upload_result = data_channel.start_upload(core_request_id).await;

            let mut session_guard = session.lock().await;
            let progress = session_guard.resource_mut_progress(data_channel.resource_id).expect("Progress not found");

            if let Err(e) = upload_result {
                log::error!(target: "peer", "Failed to send resource {:?}", e);
                progress.fail(e.to_string());
                shell_runtime.msg_from_native_bg(serialize(&MessageToShell::HandleResponse(
                    core_request_id,
                    CoreOperationOutput::Transfer(TransferOperationOutput::TransferResourceProgressUpdate(progress.clone()))
                )));

                drop(session_guard);
            } else {
                shell_runtime.msg_from_native_bg(serialize(&MessageToShell::HandleResponse(
                    core_request_id,
                    CoreOperationOutput::Transfer(TransferOperationOutput::TransferResourceProgressUpdate(progress.clone()))
                )));
            }

            let _graceful_close_in_bg = data_channel.graceful_close();
        }
    }

    pub async fn answer_session_request(
        &self,
        core_request_id: u32,
        out_session: TransferSession,
        request_id: String,
        response: Response
    ) -> Result<TransferSessionStatus, PeerErrors> {
        let session_id = out_session.order_id;
        log::info!(target: "peer", "Answering session request {:?}", session_id);
        let session = Arc::new(Mutex::new(out_session));
        self.active_sessions.lock().await.insert(session_id, Arc::downgrade(&session.clone()));
        let msg_channel = self.connection.msg_channel.get().unwrap();
        let is_accepted = match response {
            Response::TransferResponse(_) => true,
            _ => false
        };

        log::info!(target: "peer", "Sending response to peer {:?}", self.peer.id());

        // Send response first
        let _ = msg_channel.send_and_forget(PeerMessageBody {
            request_id,
            request: None,
            response: Some(response)
        })?;

        if !is_accepted {
            return Ok(TransferSessionStatus::Canceled);
        }

        // Then handle download
        let result = loop {
            let session_guard = session.lock().await;
            if session_guard.is_completed() {
                break Ok(session_guard.status());
            }

            let Some(next_resource) = session_guard.get_next_download_resource() else {
                log::info!(target: "peer", "No more resources to download");
                break Ok(session_guard.status());
            };

            let order_id = next_resource.order_id;

            drop(session_guard);

            let data_channel = match DataChannel::stream_resource(
                &self.connection,
                self.shell_runtime.clone(),
                self.throughput_controller.clone(),
                Arc::downgrade(&session),
                order_id
            )
            .await
            {
                Ok(data_channel) => data_channel,
                Err(e) => {
                    session.lock().await.force_complete(format!("Failed to stream resource: {e:?}"));
                    break Err(e.into());
                }
            };

            let shell_runtime = self.shell_runtime.clone();
            let result = data_channel.start_download(core_request_id).await;
            let mut session_guard = session.lock().await;
            log::info!(target: "nearby", "Completed resource {:?}", order_id);
            let progress = session_guard.resource_mut_progress(order_id).expect("Progress not found");
            match result {
                Ok(_) => {
                    progress.success();
                    let msg = CoreOperationOutput::Transfer(TransferOperationOutput::TransferResourceProgressUpdate(progress.clone()));

                    shell_runtime
                        .msg_from_native(serialize(&MessageToShell::HandleResponse(core_request_id, msg)))
                        .await;
                }
                Err(e) => {
                    progress.fail(e.to_string());
                    let msg = CoreOperationOutput::Transfer(TransferOperationOutput::TransferResourceProgressUpdate(progress.clone()));

                    shell_runtime
                        .msg_from_native(serialize(&MessageToShell::HandleResponse(core_request_id, msg)))
                        .await;
                }
            };

            drop(session_guard);

            let _ignore = data_channel.close().await;
        };

        self.active_sessions.lock().await.remove(&session_id);
        result
    }

    pub async fn close(&self) {
        let _ = self.connection.close().await;
        let mut active_session_ids = vec![];
        for (_, session) in self.active_sessions.lock().await.iter_mut() {
            if let Some(session) = session.upgrade() {
                active_session_ids.push(session.lock().await.order_id);
            }
        }

        for session_id in active_session_ids {
            log::info!(target: "peer", "Stopping session {:?}", session_id);
            self.stop_session(session_id).await;
        }
    }

    pub async fn send_resource_thumbnail(&self, session_id: u64, resource_id: u64, file_path: &str) -> Result<(), PeerErrors> {
        let max_chunk_size = 60 * 1024 - std::mem::size_of::<PeerRequest>();
        let thumbnail_send_timeout = Some(Duration::from_secs(5));
        let thumbnail_recv_timeout = Some(Duration::from_secs(8));

        let file = File::existing(file_path.to_owned())
            .await
            .map_err(|e| PeerErrors::FailedToSendResourceThumbnail(format!("Failed to get file: {e:?}")))?;

        let buffer = file
            .read()
            .await
            .map_err(|e| PeerErrors::FailedToSendResourceThumbnail(format!("Failed to read file: {e:?}")))?;

        log::info!(target: "peer", "Sending resource thumbnail {:?} size {}", resource_id, buffer.len());

        if buffer.len() <= max_chunk_size {
            let msg = ResourceThumbnailMessage {
                session_id: session_id as i64,
                resource_id: resource_id as i64,
                data_length: buffer.len() as i64,
                current_index: 0,
                data: Some(resource_thumbnail_message::Data::Png(buffer))
            };

            let _ = self
                .connection
                .send::<VoidResponseMessage>(
                    Request::ResourceThumbnailFullfill(msg),
                    thumbnail_send_timeout,
                    thumbnail_recv_timeout
                )
                .await??;
        } else {
            // Split the thumbnail into chunks
            let chunks = buffer.chunks(max_chunk_size);
            let mut current_index = 0;
            for (i, chunk) in chunks.enumerate() {
                let msg = ResourceThumbnailMessage {
                    session_id: session_id as i64,
                    resource_id: resource_id as i64,
                    data_length: buffer.len() as i64,
                    current_index: current_index as i64,
                    data: Some(resource_thumbnail_message::Data::Png(chunk.to_vec()))
                };

                let _ = self
                    .connection
                    .send::<VoidResponseMessage>(
                        Request::ResourceThumbnailFullfill(msg),
                        thumbnail_send_timeout,
                        thumbnail_recv_timeout
                    )
                    .await??;

                log::info!(
                    target: "peer",
                    "Sent resource thumbnail chunk {}/{} for resource {:?}, size {}",
                    i + 1,
                    buffer.chunks(max_chunk_size).len(),
                    resource_id,
                    chunk.len()
                );

                current_index += chunk.len();
            }
        }

        log::info!(target: "peer", "Completed sending resource thumbnail {:?}", resource_id);

        Ok(())
    }

    pub async fn stop_session(&self, session_id: u64) {
        let mut active_sessions = self.active_sessions.lock().await;
        if let Some(session) = active_sessions.remove(&session_id).and_then(|s| s.upgrade()) {
            let request = Request::CancelRequest(CancelTransferSessionRequest {
                session_id: session_id as i64
            });

            let mut session_guard = session.lock().await;
            log::info!(target: "peer", "Stopping session: {:?}", session_id);
            let _ = self.connection.send_request_and_forget(request);
            session_guard.cancel();
        }
    }
}

impl Deref for PeerCommunication {
    type Target = ConnectionWebRtc;

    fn deref(&self) -> &Self::Target {
        &self.connection
    }
}

impl Drop for PeerCommunication {
    fn drop(&mut self) {
        let runtime = self.shell_runtime.clone();
        log::info!(target: "peer", "Dropping peer communication");
        let connection = self.connection.clone();

        if let Some(core_request_id) = self.peer_event_request_id.get().copied() {
            log::info!(target: "peer", "Sending leaved message to peer {:?}", self.peer.id());
            let leaved_msg = CoreOperationOutput::P2P(P2POperationOutput::PeerDisconnected());
            spawn(async move {
                runtime.msg_from_native_bg(serialize(&MessageToShell::HandleResponse(core_request_id, leaved_msg)));
                // For some reason, this close will be hangup randomly
                // we need to monitor if it could serious memory leak or performance issue
                let _ = connection.close().await;
            });
        }
    }
}
