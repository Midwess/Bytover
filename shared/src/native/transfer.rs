use std::sync::Arc;

use tokio::sync::{broadcast, Mutex};

use crate::app::operations::transfer::{TransferOperation, TransferOperationOutput};
use crate::native::nearby::server::HttpServer;
use crate::network::webrtc::broadcast::BroadcastWebRtc;

pub struct TransferNative {
    pub broadcast: Arc<BroadcastWebRtc>,
}

impl TransferNative {
    pub async fn handle(&self, effect: TransferOperation) -> TransferOperationOutput {
        match effect {
            TransferOperation::StartNearbyServer => {
                let result = self.broadcast.start().await;
                log::info!(target: "transfer", "Start nearby server result: {:?}", result);
                TransferOperationOutput::StartNearbyServer
            }
            TransferOperation::StopNearbyServer => {
                TransferOperationOutput::StopNearbyServer
            }
        }
    }
}
