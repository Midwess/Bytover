use std::sync::Arc;

use shared::shell::executor::p2p::P2PNativeExecutor;
use n0_future::task::spawn;

use crate::webrtc::server::WebRtcServer;

pub struct P2PNativeExecutorImpl {
    pub web_rtc: Arc<WebRtcServer>
}

#[async_trait::async_trait]
impl P2PNativeExecutor for P2PNativeExecutorImpl {
    async fn handle(&self, request: shared::shell::api::CoreRequest, effect: shared::app::operations::p2p::P2POperation) -> Result<shared::app::operations::CoreOperationOutput, shared::errors::CoreError> {
        match effect {
            shared::app::operations::p2p::P2POperation::PeerEvents(peer_id) => {
                let web_rtc = self.web_rtc.clone();
                web_rtc.start_peer_core_stream(peer_id, request).await?;
                Ok(shared::app::operations::CoreOperationOutput::None)
            }
            shared::app::operations::p2p::P2POperation::StartNearbyServer(peer) => {
                if let Err(e) = self.web_rtc.start(request.clone(), peer).await {
                    log::error!("Failed to start nearby server: {e:?}");
                    request.response(shared::errors::CoreError::from(e)).await;
                }
                Ok(shared::app::operations::CoreOperationOutput::None)
            }
            shared::app::operations::p2p::P2POperation::SendSessionDetail {
                peer_id,
                request_id,
                session_message,
                resources,
                error
            } => {
                self.web_rtc.send_session_detail(peer_id, request_id, session_message, resources, error).await?;
                Ok(shared::app::operations::CoreOperationOutput::None)
            }
            shared::app::operations::p2p::P2POperation::StreamResourceToPeer {
                peer_id,
                session_id,
                transfer_id,
                resource
            } => {
                self.web_rtc.stream_resource_to_peer(peer_id, session_id, transfer_id, resource).await?;
                Ok(shared::app::operations::CoreOperationOutput::None)
            }
            shared::app::operations::p2p::P2POperation::SendResourceNotification {
                peer_id,
                session_id,
                resource
            } => {
                self.web_rtc.send_resource_notification(peer_id, session_id, resource).await?;
                Ok(shared::app::operations::CoreOperationOutput::None)
            }
            shared::app::operations::p2p::P2POperation::ViewSessionDetail { .. } => {
                panic!("ViewSessionDetail is not yet implemented on native")
            }
            shared::app::operations::p2p::P2POperation::DownloadResource { .. } => {
                panic!("DownloadResource is not yet implemented on native")
            }
            shared::app::operations::p2p::P2POperation::DownloadAllResources { .. } => {
                panic!("DownloadAllResources is not yet implemented on native")
            }
            shared::app::operations::p2p::P2POperation::StopNearbyServer => {
                panic!("StopNearbyServer is not yet implemented on native")
            }
            shared::app::operations::p2p::P2POperation::IsRunning => {
                panic!("IsRunning is not yet implemented on native")
            }
            shared::app::operations::p2p::P2POperation::CancelResource { .. } => {
                panic!("CancelResource is not yet implemented on native")
            }
            shared::app::operations::p2p::P2POperation::BroadcastCancelSession { .. } => {
                panic!("BroadcastCancelSession is not yet implemented on native")
            }
        }
    }
}
