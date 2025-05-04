use std::sync::Arc;

use tokio::sync::OnceCell;

use crate::app::operations::p2p::P2POperation;
use crate::app::operations::CoreOperationOutput;
use crate::network::webrtc::connection::ConnectionWebRtcErrors;
use crate::network::webrtc::web_rtc::WebRtc;
use crate::ShellRuntime;

pub struct P2PNativeExecutor {
    pub shell_runtime: OnceCell<Arc<dyn ShellRuntime>>,
    pub web_rtc: Arc<WebRtc>
}

impl P2PNativeExecutor {
    pub fn update_shell_runtime(&self, shell_runtime: &Arc<dyn ShellRuntime>) {
        if self.shell_runtime.get().is_none() {
            let _ = self.shell_runtime.set(shell_runtime.clone());
        }
    }

    pub async fn handle(&self, request_id: u32, effect: P2POperation) -> CoreOperationOutput {
        match effect {
            P2POperation::PeerEvents(peer_id) => {
                let web_rtc = self.web_rtc.clone();
                loop {
                    let Some(connection) = web_rtc
                        .get_connection(peer_id.parse().expect("Mistake, peer id must be number"))
                        .await
                        .ok()
                        .and_then(|connection| connection.upgrade())
                    else {
                        return CoreOperationOutput::ConnectionError(ConnectionWebRtcErrors::ConnectionNotFound.into());
                    };

                    match connection.next_peers_event(request_id).await {
                        Ok(_) => {
                            continue;
                        }
                        Err(e) => {
                            return CoreOperationOutput::ConnectionError(e.into());
                        }
                    }
                }
            }
            P2POperation::UpdateFindingScopes(update_finding_scopes) => {
                let web_rtc = self.web_rtc.clone();
                let result = web_rtc.update_finding_scopes(update_finding_scopes).await;
                match result {
                    Ok(_) => CoreOperationOutput::Void,
                    Err(e) => CoreOperationOutput::ConnectionError(e.into())
                }
            }
            P2POperation::StartNearbyServer(peer) => {
                let web_rtc = self.web_rtc.clone();
                let result = web_rtc.start(request_id, peer, self.shell_runtime.get().unwrap().clone()).await;
                match result {
                    Ok(_) => CoreOperationOutput::Void,
                    Err(e) => CoreOperationOutput::ConnectionError(e.into())
                }
            }
            _ => {
                panic!("Mistake, unknown operation: {:?}", effect);
            }
        }
    }
}
