use once_cell::sync::OnceCell;
use std::sync::{Arc, Mutex};
use thiserror::Error;

use shared::app::operations::p2p::{P2POperation, P2POperationOutput};
use shared::app::operations::CoreOperationOutput;
use shared::entities::peer::Peer as PeerEntity;
use shared::shell::api::CoreRequest;
use shared::shell::executor::p2p::P2PNativeExecutor;
use shared::shell::executor::transfer::WebRtc;

use crate::di_container::DiContainer;
use crate::webrtc::client::WebRtcClient;
use crate::webrtc::ice::IceAgent;
use crate::webrtc::signaling::SignalingClient;

/// P2P Executor Implementation for WASM
///
/// This implementation uses the WebRTC client for receiving data from peers.
/// WASM acts as a client that connects to a single peer (not a host).
pub struct P2PNativeExecutorImpl {
    pub web_rtc: OnceCell<Arc<WebRtc>>,
    pub client: Arc<Mutex<Option<Arc<WebRtcClient>>>>,
    pub signalling: OnceCell<SignalingClient>,
    pub current_user: OnceCell<PeerEntity>
}

/// Errors that can occur in P2P operations
#[derive(Debug, Error)]
pub enum P2PError {
    #[error("Already connected to a peer")]
    AlreadyConnected,

    #[error("Not connected to any peer")]
    NotConnected,

    #[error("WebRTC error: {0}")]
    WebRtc(String),

    #[error("Signaling error: {0}")]
    Signaling(String),

    #[error("Transfer error: {0}")]
    Transfer(String),

    #[error("Operation not supported on WASM: {0}")]
    NotSupported(String)
}

impl Default for P2PNativeExecutorImpl {
    fn default() -> Self {
        Self::new()
    }
}

impl P2PNativeExecutorImpl {
    pub fn new() -> Self {
        Self {
            web_rtc: OnceCell::new(),
            client: Arc::new(Mutex::new(None)),
            signalling: OnceCell::new(),
            current_user: OnceCell::new()
        }
    }

    pub fn get_client(&self) -> Option<Arc<WebRtcClient>> {
        self.client.lock().unwrap().clone()
    }

    pub fn set_client(&self, client: Arc<WebRtcClient>) -> Result<(), Arc<WebRtcClient>> {
        let mut current = self.client.lock().unwrap();
        if current.is_some() {
            return Err(client);
        }

        current.replace(client);
        Ok(())
    }

    pub fn set_signalling(&self, signalling: SignalingClient) -> Result<(), SignalingClient> {
        self.signalling.set(signalling)
    }

    pub fn set_current_user(&self, user: PeerEntity) -> Result<(), PeerEntity> {
        self.current_user.set(user)
    }

    pub fn is_connected(&self) -> bool {
        self.client.lock().unwrap().is_some()
    }

    pub fn current_user(&self) -> Option<&PeerEntity> {
        self.current_user.get()
    }
}

impl P2PNativeExecutorImpl {
    /// Helper to get client or return NotConnected error
    fn get_client_or_not_connected(&self) -> Result<Arc<WebRtcClient>, P2PError> {
        self.get_client().ok_or(P2PError::NotConnected)
    }
}

#[async_trait::async_trait(?Send)]
impl P2PNativeExecutor for P2PNativeExecutorImpl {
    async fn handle(&self, request: CoreRequest, effect: P2POperation) -> Result<CoreOperationOutput, shared::errors::CoreError> {
        match effect {
            P2POperation::ConnectPeer {
                signalling_key,
                signalling_route,
                current_user
            } => {
                log::info!("ConnectPeer called for WASM with key {}", signalling_key);

                if self.is_connected() {
                    return Err(P2PError::AlreadyConnected.into());
                }

                let di = DiContainer::get_instance();
                let resource_repo = di.get_local_resource_repository().await;
                let transfer_repo = di.get_transfer_session_repository();
                let signalling = di.get_signalling_client_for_route(&signalling_route);

                let client = WebRtcClient::connect(
                    current_user.clone(),
                    signalling,
                    IceAgent::new(),
                    &signalling_key,
                    resource_repo,
                    transfer_repo
                )
                .await
                .map_err(|e| P2PError::WebRtc(e.to_string()))?;

                client.start_core_stream(request.clone());
                self.set_client(client.clone()).map_err(|_| P2PError::AlreadyConnected)?;

                let peer = client
                    .peer_entity()
                    .ok_or_else(|| P2PError::WebRtc("Peer not set after signaling exchange".into()))?;
                self.set_current_user(current_user).ok();

                let client_clone = client.clone();
                let client_slot = self.client.clone();
                let request_clone = request.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    let disconnected_peer = client_clone.peer_entity();
                    let result = client_clone.run().await;
                    if let Some(peer) = disconnected_peer {
                        request_clone.response(CoreOperationOutput::P2P(P2POperationOutput::PeerDisconnected(peer))).await;
                    }
                    client_slot.lock().unwrap().take();
                    if let Err(e) = result {
                        log::error!("WebRtcClient run error: {:?}", e);
                    }
                });

                Ok(CoreOperationOutput::P2P(P2POperationOutput::PeerConnected(peer)))
            }

            P2POperation::IsRunning => {
                let running = self.is_connected();
                log::debug!("IsRunning called on WASM: {}", running);
                Ok(CoreOperationOutput::None)
            }

            P2POperation::GetPeer { peer_id } => {
                let peer = self.get_client().and_then(|client| client.peer_entity()).filter(|peer| peer.id == peer_id);
                Ok(peer.into())
            }

            P2POperation::StartNearbyServer(_) => {
                log::warn!("StartNearbyServer called on WASM - not applicable (WASM is client-only)");
                Err(P2PError::NotSupported("StartNearbyServer".into()).into())
            }

            P2POperation::StopNearbyServer => {
                log::warn!("StopNearbyServer called on WASM - not applicable");
                Err(P2PError::NotSupported("StopNearbyServer".into()).into())
            }

            P2POperation::SendSessionDetail { .. } => {
                log::debug!("SendSessionDetail called on WASM - not applicable (client-only)");
                Err(P2PError::NotSupported("SendSessionDetail".into()).into())
            }

            P2POperation::StreamResourceToPeer { .. } => {
                log::debug!("StreamResourceToPeer called on WASM - not applicable (client-only)");
                Err(P2PError::NotSupported("StreamResourceToPeer".into()).into())
            }

            P2POperation::SendResourceNotification { .. } => {
                log::debug!("SendResourceNotification called on WASM - not applicable (client-only)");
                Err(P2PError::NotSupported("SendResourceNotification".into()).into())
            }

            P2POperation::ViewSessionDetail {
                peer_id,
                order_id,
                password
            } => {
                log::info!("ViewSessionDetail called for peer {}, order {}", peer_id, order_id);

                let client = self.get_client_or_not_connected()?;

                client
                    .request_session_detail(request, order_id, password)
                    .await
                    .map_err(|e| P2PError::Transfer(e.to_string()))?;

                Ok(CoreOperationOutput::None)
            }

            P2POperation::DownloadResource {
                peer_id,
                session_id,
                resource,
                progress
            } => {
                log::info!(
                    "DownloadResource called for peer {}, session {}, resource {}",
                    peer_id,
                    session_id,
                    resource.order_id
                );

                let client = self.get_client_or_not_connected()?;
                let core_request = request.clone();

                let resource_clone = resource.clone();
                client.request_resource_download(core_request, session_id, resource_clone.clone(), progress).await?;
                Ok(CoreOperationOutput::None)
            }

            P2POperation::DownloadAllResources {
                peer_id,
                session_id,
                session_path,
                resources,
                aggregate_progress: _
            } => {
                log::info!(
                    "DownloadAllResources called for peer {}, session {}, {} resources",
                    peer_id,
                    session_id,
                    resources.len()
                );

                let client = self.get_client_or_not_connected()?;
                let core_request = request.clone();

                client.download_all_resources(core_request, session_id, session_path, resources).await?;

                Ok(CoreOperationOutput::None)
            }

            P2POperation::CancelResource {
                peer_id: _,
                session_id,
                resource_id
            } => {
                log::info!("CancelResource called for session {}, resource {}", session_id, resource_id);

                let client = self.get_client_or_not_connected()?;
                client.cancel_resource_transfer(session_id, resource_id).await;

                Ok(CoreOperationOutput::None)
            }

            P2POperation::BroadcastCancelSession { session_id, resource_id } => {
                log::info!(
                    "BroadcastCancelSession called for session {}, resource {:?}",
                    session_id,
                    resource_id
                );

                let client = self.get_client_or_not_connected()?;

                client.cancel_transfer(session_id).await;

                if let Some(res_id) = resource_id {
                    client.cancel_resource_transfer(session_id, res_id).await;
                }

                Ok(CoreOperationOutput::None)
            }
        }
    }
}

impl From<P2PError> for shared::errors::CoreError {
    fn from(err: P2PError) -> Self {
        shared::errors::CoreError::Network(err.to_string())
    }
}
