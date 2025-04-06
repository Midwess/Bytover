use std::mem;
use std::ops::Deref;
use std::sync::Arc;

use futures_util::future::join_all;
use futures_util::lock::Mutex;
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
    active_data_channels: Arc<Mutex<Vec<Arc<DataChannel>>>>,
    session_handle: JoinHandle<()>,
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

        let active_data_channels = Arc::new(Mutex::new(vec![]));
        Self::handle_data_channel(
            active_data_channels.clone(),
            shell_runtime.clone(),
            &connection,
            workdir.clone()
        );

        let session_handle = Self::handle_incomming_session(connection.clone(), shell_runtime.clone(), peer.clone()).await?;

        let me = Self {
            mine: current_peer,
            peer,
            connection,
            shell_runtime: shell_runtime.clone(),
            active_data_channels,
            session_handle,
            workdir
        };

        shell_runtime.msg_from_native(serialize(&MessageToShell::NewPeer(me.peer.clone())));

        Ok(me)
    }

    fn handle_data_channel(
        active_data_channels: Arc<Mutex<Vec<Arc<DataChannel>>>>,
        shell_runtime: Arc<dyn ShellRuntime>,
        connection: &ConnectionWebRtc,
        workdir: String
    ) {
        connection.peer_connection.on_data_channel({
            let active_data_channels = active_data_channels.clone();
            let shell_runtime = shell_runtime.clone();
            let workdir = workdir.clone();
            Box::new(move |d: Arc<webrtc::data_channel::RTCDataChannel>| {
                let active_data_channels = active_data_channels.clone();
                let shell_runtime = shell_runtime.clone();
                let workdir = workdir.clone();

                Box::pin(async move {
                    let data_channel = match DataChannel::from_channel(d, shell_runtime.clone(), workdir.clone()).await {
                        Ok(data_channel) => Arc::new(data_channel),
                        Err(e) => {
                            log::error!(target: "peer", "Failed to create data channel {:?}", e);
                            return;
                        }
                    };

                    active_data_channels.lock().await.push(data_channel.clone());

                    data_channel.start_download().await;
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

    pub async fn send_resource(&self, session_id: u64, resource: LocalResource) -> Result<(), PeerErrors> {
        let data_channel = DataChannel::stream_resource(
            session_id,
            resource,
            &self.connection,
            self.shell_runtime.clone(),
            self.workdir.clone()
        )
        .await?;

        data_channel.start_upload().await?;

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
                    shell_runtime.msg_from_native(serialize(&MessageToShell::SessionRequest(body.session.clone(), peer.clone())));
                }
            }
        });

        Ok(handle)
    }

    pub async fn stop_session(&self) {
        let mut active_data_channels = self.active_data_channels.lock().await;
        for data_channel in active_data_channels.iter_mut() {
            data_channel.stop_transfer().await;
        }

        active_data_channels.clear();
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
        self.session_handle.abort();
        self.shell_runtime.msg_from_native(serialize(&MessageToShell::PeerLeaved(self.peer.clone())));
    }
}
