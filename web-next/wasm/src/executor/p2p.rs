//! P2P Executor for WASM
//!
//! This executor handles P2P operations using the WebRTC client.
//!
//! ## Architecture
//!
//! WASM acts as a P2P client (receiver) - it connects to a single peer.
//! When `ConnectPeer(peer_id)` is called, it initiates a WebRTC connection.
//! The client instance is stored in a `OnceCell` since there's only one connection.
//!
//! ## Operations
//!
//! **Inbound (receiving from peer)**:
//! - `ConnectPeer` - Initiate WebRTC connection to peer
//! - `ViewSessionDetail` - Request session info from peer
//! - `DownloadResource` - Download a resource from peer
//! - `DownloadAllResources` - Download all resources in a session
//! - `CancelResource` - Cancel an in-progress download

use once_cell::sync::OnceCell;
use std::sync::Arc;
use thiserror::Error;

use shared::shell::executor::p2p::P2PNativeExecutor;
use shared::shell::api::CoreRequest;
use shared::app::operations::p2p::P2POperation;
use shared::app::operations::CoreOperationOutput;
use shared::entities::peer::Peer as PeerEntity;
use shared::shell::executor::transfer::WebRtc;

use crate::webrtc::client::WebRtcClient;
use crate::webrtc::ice::IceAgent;
use crate::webrtc::signaling::SignalingClient;
use crate::di_container::DiContainer;

/// P2P Executor Implementation for WASM
///
/// This implementation uses the WebRTC client for receiving data from peers.
/// WASM acts as a client that connects to a single peer (not a host).
///
/// The client is stored in a `OnceCell` since there's only one active connection.
pub struct P2PNativeExecutorImpl {
    /// WebRTC stub from shared (used for trait compatibility)
    pub web_rtc: OnceCell<Arc<WebRtc>>,

    /// WebRTC client for WASM-specific receiving operations
    pub client: OnceCell<Arc<WebRtcClient>>,

    /// Current user identity (for introduce handshake)
    pub current_user: OnceCell<PeerEntity>,
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
    NotSupported(String),
}

impl P2PNativeExecutorImpl {
    pub fn new() -> Self {
        Self {
            web_rtc: OnceCell::new(),
            client: OnceCell::new(),
            current_user: OnceCell::new(),
        }
    }

    /// Get the WebRtcClient, if connected
    pub fn get_client(&self) -> Option<Arc<WebRtcClient>> {
        self.client.get().cloned()
    }

    /// Set the client after connection is established
    pub fn set_client(&self, client: Arc<WebRtcClient>) -> Result<(), Arc<WebRtcClient>> {
        self.client.set(client)
    }

    /// Set the current user identity
    pub fn set_current_user(&self, user: PeerEntity) -> Result<(), PeerEntity> {
        self.current_user.set(user)
    }

    /// Check if connected to a peer
    pub fn is_connected(&self) -> bool {
        self.client.get().is_some()
    }

    /// Get the current user identity
    pub fn current_user(&self) -> Option<&PeerEntity> {
        self.current_user.get()
    }
}

impl P2PNativeExecutorImpl {
    /// Helper to get client or return NotConnected error
    fn get_client_or_not_connected(&self) -> Result<Arc<WebRtcClient>, P2PError> {
        self.client.get().cloned().ok_or(P2PError::NotConnected)
    }
}

#[async_trait::async_trait(?Send)]
impl P2PNativeExecutor for P2PNativeExecutorImpl {
    async fn handle(&self, _request: CoreRequest, effect: P2POperation) -> Result<CoreOperationOutput, shared::errors::CoreError> {
        match effect {
            // === Connection Management ===

            P2POperation::ConnectPeer(peer_id) => {
                log::info!("ConnectPeer called for WASM to peer {}", peer_id);

                if self.client.get().is_some() {
                    log::warn!("Already connected to a peer, ignoring ConnectPeer for {}", peer_id);
                    return Err(P2PError::AlreadyConnected.into());
                }

                let di = DiContainer::get_instance();
                let resource_repo = di.get_local_resource_repository().await;
                let transfer_repo = di.get_transfer_session_repository();

                let client = WebRtcClient::connect(
                    SignalingClient::new("http://localhost:3000"),
                    IceAgent::new(),
                    &peer_id,
                    resource_repo,
                    transfer_repo,
                )
                .await
                .map_err(|e| P2PError::WebRtc(e.to_string()))?;

                self.set_client(client.clone()).map_err(|_| P2PError::AlreadyConnected)?;

                let client_clone = client.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    if let Err(e) = client_clone.run().await {
                        log::error!("WebRtcClient run error: {:?}", e);
                    }
                });

                Ok(CoreOperationOutput::None)
            }

            P2POperation::IsRunning => {
                let running = self.client.get().is_some();
                log::debug!("IsRunning called on WASM: {}", running);
                Ok(CoreOperationOutput::None)
            }

            // === Not applicable to WASM (host-only operations) ===

            P2POperation::StartNearbyServer(_) => {
                log::warn!("StartNearbyServer called on WASM - not applicable (WASM is client-only)");
                Err(P2PError::NotSupported("StartNearbyServer".into()).into())
            }

            P2POperation::StopNearbyServer => {
                log::warn!("StopNearbyServer called on WASM - not applicable");
                Err(P2PError::NotSupported("StopNearbyServer".into()).into())
            }

            // === Outbound Operations (WASM doesn't send these) ===

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

            // === Inbound Operations (downloading from peer) ===

            P2POperation::ViewSessionDetail { peer_id, order_id, password } => {
                log::info!("ViewSessionDetail called for peer {}, order {}", peer_id, order_id);

                let client = self.get_client_or_not_connected()?;

                match client.request_session_detail(order_id, password).await {
                    Ok(_session) => {
                        log::info!("Session detail received for order_id {}", order_id);
                        // TODO: Convert P2pTransferSessionMessage to TransferSession
                        // The conversion requires implementing From<P2pTransferSessionMessage> for TransferSession
                        // For now, return None until the conversion is implemented
                        Ok(CoreOperationOutput::None)
                    }
                    Err(e) => {
                        log::error!("Session detail failed for order_id {}: {:?}", order_id, e);
                        Err(P2PError::Transfer(e.to_string()).into())
                    }
                }
            }

            P2POperation::DownloadResource { peer_id, session_id, resource, progress: _ } => {
                log::info!("DownloadResource called for peer {}, session {}, resource {}",
                    peer_id, session_id, resource.order_id);

                let client = self.get_client_or_not_connected()?;

                // Spawn the download request - the client will receive data and write to resource repo
                // Full FEC integration and progress reporting is handled in the client's receiving loop
                let client_clone = client.clone();
                let resource_clone = resource.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    match client_clone.request_resource_download(session_id, resource_clone.order_id).await {
                        Ok(()) => {
                            log::info!("Resource download completed for resource {}", resource_clone.order_id);
                        }
                        Err(e) => {
                            log::error!("Resource download failed for resource {}: {:?}", resource_clone.order_id, e);
                        }
                    }
                });

                Ok(CoreOperationOutput::None)
            }

            P2POperation::DownloadAllResources { peer_id, session_id, session_path: _, resources, aggregate_progress: _ } => {
                log::info!("DownloadAllResources called for peer {}, session {}, {} resources",
                    peer_id, session_id, resources.len());

                let client = self.get_client_or_not_connected()?;

                // Spawn downloads for all resources in parallel
                for resource in resources.iter() {
                    let client_clone = client.clone();
                    let resource_clone = resource.clone();
                    let session_id = session_id;

                    wasm_bindgen_futures::spawn_local(async move {
                        match client_clone.request_resource_download(session_id, resource_clone.order_id).await {
                            Ok(()) => {
                                log::info!("Resource download completed for resource {}", resource_clone.order_id);
                            }
                            Err(e) => {
                                log::error!("Resource download failed for resource {}: {:?}", resource_clone.order_id, e);
                            }
                        }
                    });
                }

                Ok(CoreOperationOutput::None)
            }

            P2POperation::CancelResource { peer_id: _, session_id, resource_id } => {
                log::info!("CancelResource called for session {}, resource {}", session_id, resource_id);

                let client = self.get_client_or_not_connected()?;
                client.cancel_resource_transfer(session_id, resource_id).await;

                Ok(CoreOperationOutput::None)
            }

            P2POperation::BroadcastCancelSession { session_id, resource_id } => {
                log::info!("BroadcastCancelSession called for session {}, resource {:?}", session_id, resource_id);

                let client = self.get_client_or_not_connected()?;

                // Cancel all transfers for the session
                client.cancel_transfer(session_id).await;

                // If resource_id is provided, also cancel that specific resource
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
