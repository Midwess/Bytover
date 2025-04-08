use std::sync::Arc;

use tokio::sync::OnceCell;

use crate::app::operations::transfer::{TransferOperation, TransferOperationOutput};
use crate::app::transfer::session::{TransferProgress, TransferStatus};
use crate::network::webrtc::web_rtc::WebRtc;
use crate::ShellRuntime;

pub struct TransferNative {
    pub web_rtc: Arc<WebRtc>,
    pub shell_runtime: OnceCell<Arc<dyn ShellRuntime>>
}

impl TransferNative {
    pub fn update_shell_runtime(&self, shell_runtime: &Arc<dyn ShellRuntime>) {
        if self.shell_runtime.get().is_none() {
            let _ = self.shell_runtime.set(shell_runtime.clone());
        }
    }

    pub fn shell_runtime(&self) -> Arc<dyn ShellRuntime> {
        self.shell_runtime.get().unwrap().clone()
    }

    pub async fn handle(&self, request_id: u32, effect: TransferOperation) -> TransferOperationOutput {
        match effect {
            TransferOperation::StartNearbyServer(peer) => {
                let result = self.web_rtc.start(peer, self.shell_runtime()).await;
                log::info!(target: "transfer", "Start nearby server result: {:?}", result);
                TransferOperationOutput::StartNearbyServer
            }
            TransferOperation::StopNearbyServer => TransferOperationOutput::StopNearbyServer,
            TransferOperation::UpdateFindingScopes(scopes) => {
                let result = self.web_rtc.update_finding_scopes(scopes).await;
                log::info!(target: "transfer", "Update finding scopes result: {:?}", result);
                TransferOperationOutput::UpdateFindingScopes
            }
            TransferOperation::SendSession(session) => {
                let web_rtc = self.web_rtc.clone();
                let result = web_rtc.send_session(session).await;
                log::info!(target: "transfer", "Transfer result: {:?}", result);

                TransferOperationOutput::SendSession
            }
            TransferOperation::SendResource(peer_id, resource) => {
                let web_rtc = self.web_rtc.clone();
                let resource_order_id = resource.order_id;
                let result = web_rtc.send_resource(peer_id, request_id, resource).await;

                match result {
                    Ok(_) => TransferOperationOutput::SendResourceProgressUpdate(TransferProgress {
                        percentage: 1.0,
                        resource_order_id,
                        status: TransferStatus::Success
                    }),
                    Err(error) => TransferOperationOutput::SendResourceProgressUpdate(TransferProgress {
                        percentage: 1.0,
                        resource_order_id,
                        status: TransferStatus::Fail(error.to_string())
                    })
                }
            }
            TransferOperation::DownloadResources(peer_id, resources) => {
                let web_rtc = self.web_rtc.clone();
                let result = web_rtc.download_resource(peer_id, request_id, resources).await;
                log::info!(target: "transfer", "Download resources result: {:?}", result);

                TransferOperationOutput::DownloadResourceProgressUpdate(TransferProgress {
                    percentage: 1.0,
                    resource_order_id: 0,
                    status: TransferStatus::Success
                })
            }
        }
    }
}
