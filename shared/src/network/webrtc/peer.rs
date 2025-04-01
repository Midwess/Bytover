use std::mem;
use std::ops::Deref;
use std::sync::Arc;

use schema::devlog::bitbridge::peer_message::{Request, Response};
use schema::devlog::bitbridge::{IntroduceRequest, IntroduceResponse, PeerErrors as PeerSchemaErrors};
use thiserror::Error;

use crate::entities::peer::Peer as PeerEntity;
use crate::native::message_to_shell::MessageToShell;
use crate::{serialize, ShellRuntime};

use super::connection::{ConnectionWebRtc, ConnectionWebRtcErrors};

#[derive(Debug, Error)]
pub enum PeerErrors {
    #[error("Failed to connect to peer {:?}", .0)]
    ConnectionError(#[from] ConnectionWebRtcErrors),
    #[error("Peer response error {:?}", .0)]
    ResponseError(#[from] PeerSchemaErrors),
    #[error("No response from peer")]
    NoResponseFromPeer
}

// A higher level that utilize the WebRtc connection
// To develop a transferable peer-to-peer logic
pub struct PeerCommunication {
    mine: PeerEntity,
    peer: PeerEntity,
    connection: ConnectionWebRtc,
    shell_runtime: Arc<dyn ShellRuntime>
}

impl PeerCommunication {
    pub async fn upgrade(
        connection: ConnectionWebRtc,
        current_peer: PeerEntity,
        peer_id: u128,
        shell_runtime: Arc<dyn ShellRuntime>
    ) -> Result<Self, PeerErrors> {
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
                    let peer = introduction.mine.clone().into();
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

        let me = Self {
            mine: current_peer,
            peer,
            connection,
            shell_runtime: shell_runtime.clone()
        };

        shell_runtime.msg_from_native(serialize(&MessageToShell::NewPeer(me.peer.clone())));

        Ok(me)
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
        self.shell_runtime.msg_from_native(serialize(&MessageToShell::PeerLeaved(self.peer.clone())));
    }
}
