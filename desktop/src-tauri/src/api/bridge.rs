use crux_core::middleware::Layer;
use n0_future::task::JoinHandle;
use shared::app::AppEvent;
use shared::app::operations::CoreOperationOutput;
use shared::shell::api::CoreBridge;
use crate::CORE;

pub struct BridgeImpl {}

#[async_trait::async_trait]
impl CoreBridge for BridgeImpl {
    fn response(&self, request_id: u32, response: CoreOperationOutput) -> JoinHandle<()> {
        let effects = CORE.resolve(response);
        todo!()
    }

    async fn response_throttle(&self, request_id: u32, response: CoreOperationOutput) {
        todo!()
    }

    async fn notify(&self, event: AppEvent) {
        todo!()
    }
}