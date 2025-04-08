use std::mem;
use std::ops::Deref;
use std::sync::Arc;

use futures_util::future::join_all;
use schema::devlog::bitbridge::peer_message_body::{Request, Response};
use schema::devlog::bitbridge::{
    IntroduceRequest,
    IntroduceResponse,
    PeerErrors as PeerSchemaErrors,
    TransferRequest,
    TransferResponse,
    TransferSessionMessage
};
use thiserror::Error;
use tokio::spawn;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;

use crate::app::file_system::file::LocalResource;
use crate::app::transfer::session::TransferSession;
use crate::entities::peer::Peer as PeerEntity;
use crate::native::message_to_shell::MessageToShell;
use crate::{serialize, ShellRuntime};

use super::connection::{ConnectionWebRtc, ConnectionWebRtcErrors};
use super::data_channel::{DataChannel, DataChannelError};

#[derive(Debug, Error)]
pub enum PeerErrors {
    #[error("Failed to connect to peer {:?}", .0)]
    ConnectionError(#[from] ConnectionWebRtcErrors),
    #[error("Peer response error {:?}", .0)]
    ResponseError(#[from] PeerSchemaErrors),
    #[error("No response from peer")]
    NoResponseFromPeer,
    #[error("Channel error {:?}", .0)]
    ChannelError(#[from] DataChannelError)
}

// A higher level that utilize the WebRtc connection
// To develop a transferable peer-to-peer logic
pub struct PeerCommunication {
    mine: PeerEntity,
    peer: PeerEntity,
    connection: Arc<ConnectionWebRtc>,
    shell_runtime: Arc<dyn ShellRuntime>,
    data_channel_tx: broadcast::Sender<Arc<DataChannel>>,
    workdir: String
}

impl PeerCommunication {
    pub async fn upgrade(
        connection: ConnectionWebRtc,
        current_peer: PeerEntity,
        peer_id: u128,
        shell_runtime: Arc<dyn ShellRuntime>,
        workdir: String
    ) -> Result<Self, PeerErrors> {
        let connection = Arc::new(connection);
        let peer = if current_peer.id() < peer_id {
            let introduce_request = IntroduceRequest {
                mine: current_peer.clone().into()
            };

            let response = connection.send::<IntroduceResponse>(Request::IntroduceRequest(introduce_request)).await??;
            response.peer.into()
        } else {
            let mut peer_result = None;
            while let Ok(request) = connection.next_request().await {
                if let Request::IntroduceRequest(introduction) = request.message() {
                    let peer: PeerEntity = introduction.mine.clone().into();
                    request
                        .resolve(Response::IntroduceResponse(IntroduceResponse {
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

        let (data_channel_tx, _) = broadcast::channel(16);

        let me = Self {
            mine: current_peer,
            peer,
            connection,
            shell_runtime: shell_runtime.clone(),
            data_channel_tx,
            workdir
        };

        me.handle_data_channel();

        shell_runtime.msg_from_native(serialize(&MessageToShell::NewPeer(me.peer.clone()))).await;

        Ok(me)
    }

    fn handle_data_channel(&self) {
        self.connection.peer_connection.on_data_channel({
            let data_channel_tx = self.data_channel_tx.clone();
            let shell_runtime = self.shell_runtime.clone();

            Box::new(move |d: Arc<webrtc::data_channel::RTCDataChannel>| {
                let data_channel_tx = data_channel_tx.clone();
                let shell_runtime = shell_runtime.clone();

                Box::pin(async move {
                    let data_channel = match DataChannel::from_channel(d, shell_runtime.clone()).await {
                        Ok(data_channel) => Arc::new(data_channel),
                        Err(e) => {
                            log::error!(target: "peer", "Failed to create data channel {:?}", e);
                            return;
                        }
                    };

                    let _ = data_channel_tx.send(data_channel.clone());
                })
            })
        });
    }

    pub async fn send_session(&self, session: TransferSession) -> Result<(), PeerErrors> {
        let transfer_session = TransferSessionMessage {
            order_id: session.order_id,
            resources: join_all(session.resources.iter().map(|r| r.to_proto())).await.into_iter().collect()
        };

        let request = Request::TransferRequest(TransferRequest { session: transfer_session });

        let _: TransferResponse = self.connection.send(request).await??;

        Ok(())
    }

    pub async fn download_resource(&self, core_request_id: u32, mut resources: Vec<LocalResource>) -> Result<(), PeerErrors> {
        let mut subscription = self.data_channel_tx.subscribe();
        while let Ok(data_channel) = subscription.recv().await {
            let Some(found_index) = resources.iter().position(|r| r.order_id == data_channel.resource_id) else {
                continue;
            };

            let resource = resources.remove(found_index);
            data_channel.start_download(core_request_id, resource).await?;
        }

        Ok(())
    }

    pub async fn send_resource(&self, core_request_id: u32, resource: LocalResource) -> Result<(), PeerErrors> {
        let data_channel = DataChannel::stream_resource(&resource, &self.connection, self.shell_runtime.clone()).await?;

        data_channel.start_upload(core_request_id, resource).await?;

        Ok(())
    }

    pub async fn handle_incomming_session(
        connection: Arc<ConnectionWebRtc>,
        shell_runtime: Arc<dyn ShellRuntime>,
        peer: PeerEntity
    ) -> Result<JoinHandle<()>, PeerErrors> {
        let handle = spawn(async move {
            while let Ok(request) = connection.next_request().await {
                if let Request::TransferRequest(body) = request.message() {
                    shell_runtime
                        .msg_from_native(serialize(&MessageToShell::SessionRequest(body.session.clone(), peer.clone())))
                        .await;
                    request.resolve(Response::TransferResponse(TransferResponse {})).await;
                }
            }
        });

        Ok(handle)
    }

    pub async fn stop_session(&self) {
        // let mut active_data_channels = self.active_data_channels.lock().await;
        // for data_channel in active_data_channels.iter_mut() {
        //     data_channel.stop_transfer().await;
        // }

        // active_data_channels.clear();
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
        let peer = self.peer.clone();
        spawn(async move {
            runtime.msg_from_native(serialize(&MessageToShell::PeerLeaved(peer))).await;
        });
    }
}
