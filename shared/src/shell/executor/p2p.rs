use crate::app::operations::p2p::P2POperation;
use crate::app::operations::CoreOperationOutput;
use crate::errors::CoreError;
use crate::shell::api::CoreRequest;

#[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
#[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
pub trait P2PNativeExecutor: Send + Sync {
    async fn handle(&self, request: CoreRequest, effect: P2POperation) -> Result<CoreOperationOutput, CoreError>;
}
