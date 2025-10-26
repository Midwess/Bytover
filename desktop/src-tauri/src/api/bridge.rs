use crate::{process_effects, process_event, CORE};
use shared::app::operations::CoreOperationOutput;
use shared::app::AppEvent;
use shared::shell::api::{CoreBridge, CruxRequest};
use tauri::AppHandle;

pub struct BridgeImpl {
    pub app_handle: AppHandle
}

#[async_trait::async_trait]
impl CoreBridge for BridgeImpl {
    async fn response(&self, request: &mut CruxRequest, response: CoreOperationOutput) {
        let CruxRequest::RequestHandle(handle) = request else {
            panic!("Invalid request");
        };

        let Ok(effects) = CORE.resolve(handle, response) else {
            return;
        };

        process_effects(effects, self.app_handle.clone()).await;
    }

    // The response throttle in desktop don't need to be throttle
    // because there are no FFI bridge in desktop, so performance is not a concern.
    async fn response_throttle(&self, request: &mut CruxRequest, response: CoreOperationOutput) {
        self.response(request, response).await;
    }

    async fn notify(&self, event: AppEvent) {
        process_event(event, self.app_handle.clone()).await;
    }
}
