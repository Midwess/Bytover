use std::mem;
use std::ops::Deref;
use std::sync::Arc;
use std::time::Duration;

use futures_util::future::join_all;
use schema::devlog::bitbridge::peer_message_body::{Request, Response};
use schema::devlog::bitbridge::{
    IntroduceRequestMessage,
    IntroduceResponseMessage,
    PeerErrorsMessage,
    PeerMessageBody,
    TransferRequestMessage,
    TransferResponseMessage,
    TransferSessionMessage
};
use thiserror::Error;
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::{broadcast, OnceCell};
use tokio::time::timeout;
use tokio::{select, spawn};

use crate::app::file_system::file::LocalResource;
use crate::app::operations::p2p::P2POperationOutput;
use crate::app::operations::transfer::TransferOperationOutput;
use crate::app::operations::CoreOperationOutput;
use crate::app::transfer::session::{TransferProgress, TransferSession, TransferType};
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
    throughput_controller: Arc<ThroughputController>
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

            let response = connection.send::<IntroduceResponseMessage>(Request::IntroduceRequest(introduce_request)).await??;
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
            throughput_controller
        };

        me.handle_data_channel();

        Ok(me)
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
                    _ => {
                        log::warn!(target: "peer", "Unexpected request from peer {:?}", self.peer.id());
                    }
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
            Box::new(move |d: Arc<webrtc::data_channel::RTCDataChannel>| {
                let data_channel_tx = data_channel_tx.clone();
                let shell_runtime = shell_runtime.clone();
                let throughput_controller = throughput_controller.clone();
                Box::pin(async move {
                    let data_channel = match DataChannel::from_channel(d, shell_runtime.clone(), throughput_controller.clone()).await {
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

    pub async fn send_session(&self, mut session: TransferSession, core_request_id: u32) -> Result<(), PeerErrors> {
        let transfer_session = TransferSessionMessage {
            order_id: session.order_id,
            resources: join_all(session.resources.iter().map(|r| r.to_proto())).await.into_iter().collect()
        };

        log::info!(target: "peer", "Sending session to peer {:?}", self.peer.id());
        let request = Request::TransferRequest(TransferRequestMessage {
            session: transfer_session.clone()
        });

        let mut channel_subscription = self.data_channel_tx.subscribe();

        let _ = self.connection.send::<TransferResponseMessage>(request).await??;

        let resources = &mut session.resources;
        loop {
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

            let Some(found_index) = resources
                .iter()
                .position(|r| r.order_id == data_channel.resource_id && session.order_id == data_channel.session_id)
            else {
                continue;
            };

            let resource = resources.remove(found_index);
            let order_id = resource.order_id;
            let shell_runtime = self.shell_runtime.clone();
            let mut progress = TransferProgress::new(order_id, resource.size, TransferType::Send);

            if let Err(e) = data_channel.start_upload(core_request_id, resource, &mut progress).await {
                log::error!(target: "peer", "Failed to send resource {:?}", e);
                progress.fail(e.to_string());
                shell_runtime.msg_from_native_bg(serialize(&MessageToShell::HandleResponse(
                    core_request_id,
                    CoreOperationOutput::Transfer(TransferOperationOutput::TransferResourceProgressUpdate(progress))
                )));

                let _ = data_channel.close().await;
            } else {
                progress.success();
                shell_runtime.msg_from_native_bg(serialize(&MessageToShell::HandleResponse(
                    core_request_id,
                    CoreOperationOutput::Transfer(TransferOperationOutput::TransferResourceProgressUpdate(progress))
                )));
            }

            if resources.is_empty() {
                break;
            }
        }

        Ok(())
    }

    pub async fn answer_session_request(
        &self,
        core_request_id: u32,
        mut out_resources: Vec<LocalResource>,
        session_id: u64,
        request_id: String,
        response: Response
    ) -> Result<(), PeerErrors> {
        let msg_channel = self.connection.msg_channel.get().unwrap();
        let is_accepted = match response {
            Response::TransferResponse(_) => true,
            _ => false
        };

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
            let data_channel = DataChannel::stream_resource(
                &out_resources[0],
                session_id,
                &self.connection,
                self.shell_runtime.clone(),
                self.throughput_controller.clone()
            )
            .await?;

            let Some(found_index) = out_resources
                .iter()
                .position(|r| r.order_id == data_channel.resource_id && session_id == data_channel.session_id)
            else {
                continue;
            };

            let out_resource = out_resources.remove(found_index);
            let shell_runtime = self.shell_runtime.clone();
            let mut progress = TransferProgress::new(out_resource.order_id, out_resource.size, TransferType::Receive);
            match data_channel.start_download(core_request_id, &out_resource, &mut progress).await {
                Ok(_) => {
                    progress.success();
                    let msg = CoreOperationOutput::Transfer(TransferOperationOutput::TransferResourceProgressUpdate(progress));
                    shell_runtime.msg_from_native_bg(serialize(&MessageToShell::HandleResponse(core_request_id, msg)));
                }
                Err(e) => {
                    progress.fail(e.to_string());
                    let msg = CoreOperationOutput::Transfer(TransferOperationOutput::TransferResourceProgressUpdate(progress));
                    shell_runtime.msg_from_native_bg(serialize(&MessageToShell::HandleResponse(core_request_id, msg)));
                }
            };

            let _ = data_channel.close().await;

            if out_resources.is_empty() {
                break;
            }
        }

        Ok(())
    }

    pub async fn stop_session(&self) {
        // let mut active_data_channels = self.active_data_channels.lock().await;
        // for data_channel in active_data_channels.iter_mut() {
        //     data_channel.stop_transfer().await;
        // }

        // active_data_channels.clear();
    }

    pub async fn close(&self) {
        let _ = self.connection.close().await;
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
        if let Some(core_request_id) = self.peer_event_request_id.get().copied() {
            let leaved_msg = CoreOperationOutput::P2P(P2POperationOutput::PeerDisconnected());
            log::info!(target: "peer", "Sending leaved message to peer {:?}", self.peer.id());
            let connection = self.connection.clone();
            spawn(async move {
                let _ = connection.peer_connection.close().await;
                runtime.msg_from_native_bg(serialize(&MessageToShell::HandleResponse(core_request_id, leaved_msg)));
            });
        }
    }
}
