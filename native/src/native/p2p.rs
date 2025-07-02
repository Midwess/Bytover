use std::sync::Arc;

use crate::ShellRuntime;
use shared::app::operations::p2p::P2POperation;
use shared::app::operations::CoreOperationOutput;
use shared::core_transfer_protocol::webrtc::webrtc::WebRtc;

pub struct P2PNativeExecutor {
    pub shell_runtime: Arc<dyn ShellRuntime>,
    pub web_rtc: Arc<WebRtc>
}

impl P2PNativeExecutor {
    pub async fn handle(&self, request_id: u32, effect: P2POperation) -> CoreOperationOutput {
        match effect {
            P2POperation::PeerEvents(peer_id) => {
                let web_rtc = self.web_rtc.clone();
                match web_rtc.start_peer_core_stream(peer_id, request_id).await {
                    Ok(_) => CoreOperationOutput::Void,
                    Err(e) => CoreOperationOutput::ConnectionError(e.into())
                }
            }
            P2POperation::UpdateFindingScopes(update_finding_scopes) => {
                let web_rtc = self.web_rtc.clone();
                let result = web_rtc.update_finding_scopes(update_finding_scopes).await;
                CoreOperationOutput::Void
            }
            P2POperation::StartNearbyServer(peer) => {
                let web_rtc = self.web_rtc.clone();
                let result = web_rtc.start(request_id, peer).await;
                match result {
                    Ok(_) => CoreOperationOutput::Void,
                    Err(e) => CoreOperationOutput::ConnectionError(e.into())
                }
            }
        }
    }
}
