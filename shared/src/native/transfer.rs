use std::sync::Arc;

use crate::app::operations::transfer::{TransferOperation, TransferOperationOutput};
use crate::network::webrtc::broadcast::BroadcastWebRtc;

pub struct TransferNative {
    pub broadcast: Arc<BroadcastWebRtc>
}

impl TransferNative {
    pub async fn handle(&self, effect: TransferOperation) -> TransferOperationOutput {
        match effect {
            TransferOperation::StartNearbyServer(scopes) => {
                let result = self.broadcast.start(scopes).await;
                log::info!(target: "transfer", "Start nearby server result: {:?}", result);
                TransferOperationOutput::StartNearbyServer
            }
            TransferOperation::StopNearbyServer => TransferOperationOutput::StopNearbyServer,
            TransferOperation::UpdateFindingScopes(scopes) => {
                let result = self.broadcast.update_finding_scopes(scopes).await;
                log::info!(target: "transfer", "Update finding scopes result: {:?}", result);
                TransferOperationOutput::UpdateFindingScopes
            }
        }
    }
}
