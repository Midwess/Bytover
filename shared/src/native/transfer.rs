use std::sync::Arc;

use tokio::sync::OnceCell;

use crate::app::operations::transfer::{TransferOperation, TransferOperationOutput};
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

    pub async fn handle(&self, effect: TransferOperation) -> TransferOperationOutput {
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
        }
    }
}
