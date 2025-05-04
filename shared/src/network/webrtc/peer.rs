use std::collections::HashMap;
use std::mem;
use std::ops::Deref;
use std::sync::{Arc, Weak};
use std::time::Duration;

use core_services::logger::ThrottleLogger;
use futures_util::future::join_all;
use schema::devlog::bitbridge::peer_message_body::{Request, Response};
use schema::devlog::bitbridge::{
    CancelTransferSessionRequest,
    IntroduceRequestMessage,
    IntroduceResponseMessage,
    KeepAliveRequestMessage,
    PeerErrorsMessage,
    PeerMessageBody,
    TransferRequestMessage,
    TransferResponseMessage,
    TransferSessionMessage,
    VoidResponseMessage
};
use thiserror::Error;
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::{broadcast, Mutex, OnceCell};
use tokio::task::JoinHandle;
use tokio::time::timeout;
use tokio::{select, spawn};
use webrtc::data_channel::data_channel_state::RTCDataChannelState;

use crate::app::operations::p2p::P2POperationOutput;
use crate::app::operations::transfer::TransferOperationOutput;
use crate::app::operations::CoreOperationOutput;
use crate::app::transfer::session::TransferSession;
use crate::entities::peer::Peer as PeerEntity;
use crate::native::message_to_shell::MessageToShell;
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
    #[error("Error while receiving resource")]
    ErrorWhileReceivingResource(String),
    #[error("Error while sending resource")]
    ErrorWhileSendingResource(String),
    #[error("Channel error {:?}", .0)]
    ChannelError(#[from] DataChannelError)
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
    keep_alive_handle: OnceCell<(JoinHandle<()>, JoinHandle<()>)>
}

impl PeerCommunication {
    pub async fn upgrade(
        connection: ConnectionWebRtc,
        current_peer: PeerEntity,
        peer_id: u128,
        shell_runtime: Arc<dyn ShellRuntime>,
        throughput_controller: Arc<ThroughputController>
    ) -> Result<Self, PeerErrors> {
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
            while let Ok(request) = connection.next_request().await {
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

        let me = Self {
            peer_event_request_id: OnceCell::new(),
            mine: current_peer,
            peer,
            connection,
            shell_runtime: shell_runtime.clone(),
            data_channel_tx,
            throughput_controller,
            active_sessions: Arc::new(Mutex::new(HashMap::new())),
            keep_alive_handle: OnceCell::new()
        };

        me.keep_alive();
        me.handle_data_channel();

        Ok(me)
    }

    pub fn keep_alive(&self) {
        let connection = self.connection.clone();
        let active_sessions = self.active_sessions.clone();
        let throttle_logger = ThrottleLogger::new("keep_alive".to_string(), Duration::from_secs(15));
        let peer_id = self.peer.id();

        // Spawn task for sending keep-alive requests
        let sender_handle = {
            let connection = connection.clone();
            let active_sessions = active_sessions.clone();
            let keep_alive_request = Request::KeepAlive(KeepAliveRequestMessage {});
            let send_timeout = Some(Duration::from_secs(4));
            let recv_timeout = Some(Duration::from_secs(4));

            spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(3));

                loop {
                    interval.tick().await;

                    // Skip sending keep-alive if there's active data transfer
                    let mut should_skip = false;
                    for (_, session) in active_sessions.lock().await.iter_mut() {
                        if let Some(session) = session.upgrade() {
                            let session_guard = session.lock().await;
                            if session_guard.bytes_per_second() > 0 {
                                should_skip = true;
                                break;
                            }
                        }
                    }

                    if should_skip {
                        continue;
                    }

                    match connection.send::<VoidResponseMessage>(keep_alive_request.clone(), send_timeout, recv_timeout).await {
                        Ok(Ok(_)) => {
                            continue;
                        }
                        Ok(Err(e)) => {
                            log::error!(target: "peer", "Failed to send keep alive request {:?}", e);
                            break;
                        }
                        Err(e) => {
                            log::error!(target: "peer", "Failed to send keep alive request {:?}", e);
                            break;
                        }
                    }
                }

                connection.close().await;
            })
        };

        // Spawn task for handling incoming keep-alive requests
        let receiver_handle = {
            let connection = connection.clone();

            spawn(async move {
                loop {
                    match connection.next_request().await {
                        Ok(request) => {
                            if let Request::KeepAlive(_) = request.message() {
                                throttle_logger.log(format!("Received keep alive request from peer {:?}", peer_id)).await;
                                if let Err(e) = request.resolve(Response::VoidResponse(VoidResponseMessage {})).await {
                                    log::error!(target: "peer", "Failed to resolve keep alive request {:?}", e);
                                    break;
                                }
                            }
                        }
                        Err(e) => {
                            log::error!(target: "peer", "Failed to receive keep alive request {:?}", e);
                            break;
                        }
                    }
                }

                connection.close().await;
            })
        };

        // Store the sender handle - we only need one handle to cancel both tasks
        // since if either task exits, it will close the connection and cause the other to exit
        let _ = self.keep_alive_handle.set((sender_handle, receiver_handle));
    }

    pub async fn next_peers_event(&self, core_request_id: u32) -> Result<(), PeerErrors> {
        let _ = self.peer_event_request_id.set(core_request_id);

        select! {
            request = self.connection.next_request() => {
                let request = request?;
                match request.message() {
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
                        return;
                    };

                    let Some(session) = active_sessions.get(&session_id).and_then(|s| s.upgrade()) else {
                        log::warn!(target: "peer", "Session not found");
                        return;
                    };

                    let mut session_guard = session.lock().await;

                    if session_guard.is_completed() {
                        log::warn!(target: "peer", "Session is completed");
                        return;
                    }

                    let resource_progress = session_guard.resource_mut_progress(resource_id).expect("Progress not found");
                    if resource_progress.is_completed() {
                        log::warn!(target: "peer", "Resource is already completed");
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

    pub async fn send_session(&self, session: TransferSession, core_request_id: u32) -> Result<(), PeerErrors> {
        let session_id = session.order_id;
        let session = Arc::new(Mutex::new(session));

        self.active_sessions.lock().await.insert(session_id, Arc::downgrade(&session));

        let session_guard = session.lock().await;
        let transfer_session = TransferSessionMessage {
            order_id: session_guard.order_id,
            resources: join_all(session_guard.resources.iter().map(|r| r.to_proto())).await.into_iter().collect()
        };

        log::info!(target: "peer", "Sending session to peer {:?}", self.peer.id());
        let request = Request::TransferRequest(TransferRequestMessage {
            session: transfer_session.clone()
        });

        let mut channel_subscription = self.data_channel_tx.subscribe();

        let _ = self.connection.send::<TransferResponseMessage>(request, None, None).await??;

        drop(session_guard);

        loop {
            let session_guard = session.lock().await;
            if session_guard.is_completed() {
                break;
            }

            drop(session_guard);

            let data_channel = match timeout(Duration::from_secs(15), channel_subscription.recv()).await {
                Ok(Ok(data_channel)) => {
                    // Make sure the channel is not closed automatically
                    // while the buffer is not empty
                    data_channel.auto_close(false);
                    data_channel
                }
                Err(_) => {
                    return Err(PeerErrors::NoResponseFromPeer);
                }
                Ok(Err(e)) => match e {
                    RecvError::Closed => {
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

                let _ = data_channel.close().await;
            } else {
                shell_runtime.msg_from_native_bg(serialize(&MessageToShell::HandleResponse(
                    core_request_id,
                    CoreOperationOutput::Transfer(TransferOperationOutput::TransferResourceProgressUpdate(progress.clone()))
                )));
            }
        }

        self.active_sessions.lock().await.remove(&session_id);

        Ok(())
    }

    pub async fn answer_session_request(
        &self,
        core_request_id: u32,
        out_session: TransferSession,
        request_id: String,
        response: Response
    ) -> Result<(), PeerErrors> {
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
        let _ = msg_channel
            .send_and_forget(PeerMessageBody {
                request_id,
                request: None,
                response: Some(response)
            })?
            .await;

        if !is_accepted {
            return Ok(());
        }

        loop {
            let session_guard = session.lock().await;
            if session_guard.is_completed() {
                break;
            }

            let Some(next_resource) = session_guard.get_next_download_resource() else {
                log::info!(target: "peer", "No more resources to download");
                break;
            };

            let order_id = next_resource.order_id;

            drop(session_guard);

            let data_channel = DataChannel::stream_resource(
                &self.connection,
                self.shell_runtime.clone(),
                self.throughput_controller.clone(),
                Arc::downgrade(&session),
                order_id
            )
            .await?;

            let shell_runtime = self.shell_runtime.clone();
            let result = data_channel.start_download(core_request_id).await;
            let mut session_guard = session.lock().await;
            log::info!(target: "nearby", "Completed resource {:?}", order_id);
            let progress = session_guard.resource_mut_progress(order_id).expect("Progress not found");
            match result {
                Ok(_) => {
                    progress.success();
                    let msg = CoreOperationOutput::Transfer(TransferOperationOutput::TransferResourceProgressUpdate(progress.clone()));
                    shell_runtime.msg_from_native_bg(serialize(&MessageToShell::HandleResponse(core_request_id, msg)));
                }
                Err(e) => {
                    progress.fail(e.to_string());
                    let msg = CoreOperationOutput::Transfer(TransferOperationOutput::TransferResourceProgressUpdate(progress.clone()));
                    shell_runtime.msg_from_native_bg(serialize(&MessageToShell::HandleResponse(core_request_id, msg)));
                }
            };

            drop(session_guard);

            let _ = data_channel.close().await;
        }

        self.active_sessions.lock().await.remove(&session_id);

        Ok(())
    }

    pub async fn stop_session(&self, session_id: u64) {
        let mut active_sessions = self.active_sessions.lock().await;
        if let Some(session) = active_sessions.remove(&session_id).and_then(|s| s.upgrade()) {
            let request = Request::CancelRequest(CancelTransferSessionRequest {
                session_id: session_id as i64
            });

            let mut session_guard = session.lock().await;
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

        if let Some((sender_handle, receiver_handle)) = self.keep_alive_handle.get() {
            sender_handle.abort();
            receiver_handle.abort();
        }
    }
}
