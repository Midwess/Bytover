use crate::app::operations::p2p::P2POperation;
use crate::app::operations::CoreOperationOutput;
use crate::errors::CoreError;
use crate::protocol::webrtc::webrtc::WebRtc;
use crate::shell::api::CoreRequest;
use n0_future::task::spawn;
use std::sync::Arc;

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
                self.web_rtc().download_resource(peer_id, request, session_id, resource, progress).await?;
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
                self.web_rtc()
                    .download_all_resources(peer_id, request, session_id, session_path, resources, aggregate_progress)
                    .await?;
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
