use shared::app::AppEvent;
use shared::app::operations::CoreOperationOutput;
use shared::shell::api::{CoreBridge, CruxRequest};
use crate::{process_effects, process_event, CORE};

pub struct BridgeImpl {}

#[async_trait::async_trait]
impl CoreBridge for BridgeImpl {
    async fn response(&self, request: &mut CruxRequest, response: CoreOperationOutput) {
        let CruxRequest::RequestHandle(handle) = request else {
            panic!("Invalid request");
        };

        let Ok(effects) = CORE.resolve(handle, response) else {
            return;
        };

        process_effects(effects).await;
    }

    async fn response_throttle(
        &self,
        request: &mut CruxRequest,
        response: CoreOperationOutput
    ) {
        self.response(request, response).await;
    }

    async fn notify(&self, event: AppEvent) {
        process_event(event).await;
    }
}
