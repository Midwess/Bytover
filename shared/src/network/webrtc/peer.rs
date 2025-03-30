use std::mem;
use std::ops::Deref;
use std::sync::Arc;

use schema::devlog::bitbridge::peer_message::{Request, Response};
use schema::devlog::bitbridge::{IntroduceRequest, IntroduceResponse, PeerErrors as PeerSchemaErrors};
use thiserror::Error;

use crate::entities::peer::Peer as PeerEntity;
use crate::ShellRuntime;

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
        if current_peer.id() > peer_id {
            let introduce_request = IntroduceRequest {
                mine: current_peer.clone().into()
            };

            let response = connection.send::<IntroduceResponse>(Request::IntroduceRequest(introduce_request)).await??;
            let peer = response.peer.into();

            log::info!(target: "peer", "Connected to peer {:?}, size = {}", peer, mem::size_of::<PeerCommunication>());
            Ok(Self {
                mine: current_peer,
                peer,
                connection,
                shell_runtime
            })
        } else {
            while let Ok(request) = connection.next_request().await {
                if let Request::IntroduceRequest(introduction) = request.message() {
                    let peer = introduction.mine.clone().into();
                    request
                        .resolve(Response::IntroduceResponse(IntroduceResponse {
                            peer: current_peer.clone().into()
                        }))
                        .await?;
                    log::info!(target: "peer", "Connected to peer {:?}, size = {}", peer, mem::size_of::<PeerCommunication>());
                    return Ok(Self {
                        mine: current_peer,
                        peer,
                        connection,
                        shell_runtime
                    })
                }
            }

            Err(PeerErrors::NoResponseFromPeer)
        }
    }
}

impl Deref for PeerCommunication {
    type Target = ConnectionWebRtc;

    fn deref(&self) -> &Self::Target {
        &self.connection
    }
}
