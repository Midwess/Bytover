// Stub WebRtc - actual implementation in disabled webrtc module
use crate::app::operations::p2p::P2POperation;
use crate::app::operations::CoreOperationOutput;
use crate::errors::CoreError;
use crate::shell::api::CoreRequest;
use n0_future::task::spawn;
use std::sync::Arc;
use crate::entities::local_resource::LocalResource;
use crate::entities::peer::Peer;
use crate::entities::transfer_session::TransferProgress;
use schema::devlog::bitbridge::P2pTransferSessionMessage;

#[derive(Clone)]
pub struct WebRtc;

impl WebRtc {
    pub async fn start_peer_core_stream(&self, _peer_id: String, _request: CoreRequest) -> Result<(), CoreError> {
        Ok(())
    }

    pub async fn update_finding_scopes(&self, _scopes: Vec<String>) -> Result<(), CoreError> {
        Ok(())
    }

    pub async fn stop(&self) {}

    pub fn is_running(&self) -> bool {
        false
    }

    pub async fn start(&self, _request: CoreRequest, _peer: Peer) -> Result<(), CoreError> {
        Ok(())
    }

    pub async fn view_session_detail(&self, _peer_id: String, _request: CoreRequest, _order_id: u64, _password: Option<String>) -> Result<(), CoreError> {
        Ok(())
    }

    pub async fn send_session_detail(&self, _peer_id: String, _request_id: String, _session_message: Option<P2pTransferSessionMessage>, _resources: Option<Vec<LocalResource>>, _error: Option<CoreError>) -> Result<(), CoreError> {
        Ok(())
    }

    pub async fn download_resource(&self, _peer_id: String, _request: CoreRequest, _session_id: u64, _resource: LocalResource, _progress: TransferProgress) -> Result<TransferProgress, CoreError> {
        Ok(_progress)
    }

    pub async fn stream_resource_to_peer(&self, _peer_id: String, _session_id: u64, _transfer_id: u16, _resource: LocalResource) -> Result<(), CoreError> {
        Ok(())
    }

    pub async fn cancel_resource(&self, _peer_id: String, _session_id: u64, _resource_id: u64) -> Result<(), CoreError> {
        Ok(())
    }

    pub async fn broadcast_cancel_session(&self, _session_id: u64, _resource_id: Option<u64>) -> Result<(), CoreError> {
        Ok(())
    }

    pub async fn download_all_resources(&self, _peer_id: String, _request: CoreRequest, _session_id: u64, _session_path: LocalResource, _resources: Vec<LocalResource>, _aggregate_progress: TransferProgress) -> Result<TransferProgress, CoreError> {
        Ok(_aggregate_progress)
    }

    pub async fn send_resource_notification(&self, _peer_id: String, _session_id: u64, _resource: LocalResource) -> Result<(), CoreError> {
        Ok(())
    }
}

#[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
#[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
pub trait P2PNativeExecutor: Send + Sync {
    fn web_rtc(&self) -> &Arc<WebRtc>;

    async fn handle(&self, request: CoreRequest, effect: P2POperation) -> Result<CoreOperationOutput, CoreError> {
        match effect {
            P2POperation::PeerEvents(peer_id) => {
                let web_rtc = self.web_rtc().clone();
                web_rtc.start_peer_core_stream(peer_id, request).await?;
                Ok(CoreOperationOutput::None)
            }
            P2POperation::UpdateFindingScopes(update_finding_scopes) => {
                let web_rtc = self.web_rtc().clone();
                let _ = web_rtc.update_finding_scopes(update_finding_scopes).await;
                Ok(CoreOperationOutput::None)
            }
            P2POperation::StopNearbyServer => {
                self.web_rtc().stop().await;
                Ok(CoreOperationOutput::None)
            }
            P2POperation::IsRunning => Ok(CoreOperationOutput::Bool(self.web_rtc().is_running())),
            P2POperation::StartNearbyServer(peer) => {
                let web_rtc = self.web_rtc().clone();
                spawn(async move {
                    if let Err(e) = web_rtc.start(request.clone(), peer).await {
                        log::error!("Failed to start nearby server: {e:?}");
                        request.response(CoreError::from(e)).await;
                    }
                });

                Ok(CoreOperationOutput::None)
            }
            P2POperation::ViewSessionDetail {
                peer_id,
                order_id,
                password
            } => {
                self.web_rtc().view_session_detail(peer_id, request, order_id, password).await?;
                Ok(CoreOperationOutput::None)
            }
            P2POperation::SendSessionDetail {
                peer_id,
                request_id,
                session_message,
                resources,
                error
            } => {
                self.web_rtc().send_session_detail(peer_id, request_id, session_message, resources, error).await?;
                Ok(CoreOperationOutput::None)
            }
            P2POperation::DownloadResource {
                peer_id,
                session_id,
                resource,
                progress
            } => {
                let mut progress = self.web_rtc().download_resource(peer_id, request, session_id, resource, progress).await?;
                progress.success();
                Ok(CoreOperationOutput::None)
            }
            P2POperation::StreamResourceToPeer {
                peer_id,
                session_id,
                transfer_id,
                resource
            } => {
                self.web_rtc().stream_resource_to_peer(peer_id, session_id, transfer_id, resource).await?;
                Ok(CoreOperationOutput::None)
            }
            P2POperation::CancelResource {
                peer_id,
                session_id,
                resource_id
            } => {
                self.web_rtc().cancel_resource(peer_id, session_id, resource_id).await?;
                Ok(CoreOperationOutput::None)
            }
            P2POperation::BroadcastCancelSession { session_id, resource_id } => {
                self.web_rtc().broadcast_cancel_session(session_id, resource_id).await?;
                Ok(CoreOperationOutput::None)
            }
            P2POperation::DownloadAllResources {
                peer_id,
                session_id,
                session_path,
                resources,
                aggregate_progress
            } => {
                let mut progress = self.web_rtc()
                    .download_all_resources(peer_id, request, session_id, session_path, resources, aggregate_progress)
                    .await?;
                progress.success();
                Ok(CoreOperationOutput::None)
            }
            P2POperation::SendResourceNotification {
                peer_id,
                session_id,
                resource
            } => {
                self.web_rtc().send_resource_notification(peer_id, session_id, resource).await?;
                Ok(CoreOperationOutput::None)
            }
        }
    }
}
