use crate::app::operations::p2p::P2POperation;
use crate::app::operations::CoreOperationOutput;
use crate::protocol::webrtc::webrtc::WebRtc;
use n0_future::task::spawn;
use std::sync::Arc;

#[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
#[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
pub trait P2PNativeExecutor: Send + Sync {
    fn web_rtc(&self) -> &Arc<WebRtc>;

    async fn handle(&self, request_id: u32, effect: P2POperation) -> CoreOperationOutput {
        match effect {
            P2POperation::PeerEvents(peer_id) => {
                let web_rtc = self.web_rtc().clone();
                match web_rtc.start_peer_core_stream(peer_id, request_id).await {
                    Ok(_) => CoreOperationOutput::Void,
                    Err(e) => CoreOperationOutput::ConnectionError(e.into())
                }
            }
            P2POperation::UpdateFindingScopes(update_finding_scopes) => {
                log::info!("Updating finding scopes: {:?}", update_finding_scopes);
                let web_rtc = self.web_rtc().clone();
                let _ = web_rtc.update_finding_scopes(update_finding_scopes).await;
                CoreOperationOutput::Void
            }
            P2POperation::StartNearbyServer(peer) => {
                let web_rtc = self.web_rtc().clone();
                spawn(async move {
                    if let Err(e) = web_rtc.start(request_id, peer).await {
                        log::error!("Failed to start nearby server: {e:?}");
                    }
                });

                CoreOperationOutput::Void
            }
        }
    }
}
