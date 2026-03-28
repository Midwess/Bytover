use std::sync::Arc;

use crate::webrtc::server::WebRtcServer;
use n0_future::task::spawn;
use shared::app::operations::p2p::P2POperationOutput;
use shared::app::operations::CoreOperationOutput;
use shared::shell::executor::p2p::P2PNativeExecutor;

pub struct P2PNativeExecutorImpl {
    pub web_rtc: Arc<WebRtcServer>,
}

#[async_trait::async_trait]
impl P2PNativeExecutor for P2PNativeExecutorImpl {
    async fn handle(
        &self,
        request: shared::shell::api::CoreRequest,
        effect: shared::app::operations::p2p::P2POperation,
    ) -> Result<CoreOperationOutput, shared::errors::CoreError> {
        match effect {
            shared::app::operations::p2p::P2POperation::ConnectPeer { signalling_key, .. } => {
                let web_rtc = self.web_rtc.clone();
                web_rtc.start_peer_core_stream(signalling_key, request).await?;
                Ok(CoreOperationOutput::None)
            }
            shared::app::operations::p2p::P2POperation::StartNearbyServer(peer) => {
                if let Err(e) = self.web_rtc.start(request.clone(), peer).await {
                    log::error!("Failed to start nearby server: {e:?}");
                    request.response(shared::errors::CoreError::from(e)).await;
                }
                Ok(P2POperationOutput::NearbyServerStopped.into())
            }
            shared::app::operations::p2p::P2POperation::SendSessionDetail {
                peer_id,
                request_id,
                session_message,
                resources,
                error,
            } => {
                self.web_rtc.send_session_detail(peer_id, request_id, session_message, resources, error).await?;
                Ok(CoreOperationOutput::None)
            }
            shared::app::operations::p2p::P2POperation::StreamResourceToPeer {
                peer_id,
                session_id,
                transfer_id,
                resource,
            } => {
                self.web_rtc.stream_resource_to_peer(peer_id, session_id, transfer_id, resource).await?;
                Ok(CoreOperationOutput::None)
            }
            shared::app::operations::p2p::P2POperation::SendResourceNotification {
                peer_id,
                session_id,
                resource,
            } => {
                self.web_rtc.send_resource_notification(peer_id, session_id, resource).await?;
                Ok(CoreOperationOutput::None)
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
                let _ = self.web_rtc.stop();
                Ok(CoreOperationOutput::None)
            }
            shared::app::operations::p2p::P2POperation::IsRunning => Ok(self.web_rtc.is_running().into()),
            shared::app::operations::p2p::P2POperation::CancelResource { .. } => {
                panic!("CancelResource is not yet implemented on native")
            }
            shared::app::operations::p2p::P2POperation::BroadcastCancelSession { .. } => {
                panic!("BroadcastCancelSession is not yet implemented on native")
            }
        }
    }
}
