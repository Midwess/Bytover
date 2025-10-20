use crate::native::message_to_shell::MessageToShell;
use crate::{ShellRuntime, ThrottleShellRuntime};
use shared::app::operations::CoreOperationOutput;
use shared::app::AppEvent;
use shared::shell::api::{CoreBridge, CruxRequest};
use std::sync::Arc;
use std::time::Duration;

pub struct CoreBridgeImpl {
    pub shell: Arc<dyn ShellRuntime>,
    pub throttle_shell_runtime: ThrottleShellRuntime<MessageToShell>
}

impl CoreBridgeImpl {
    pub fn new(shell: Arc<dyn ShellRuntime>) -> Self {
        Self {
            throttle_shell_runtime: ThrottleShellRuntime::new(shell.clone(), Duration::from_millis(500)),
            shell
        }
    }
}

#[async_trait::async_trait]
impl CoreBridge for CoreBridgeImpl {
    async fn response(&self, request: &mut CruxRequest, response: CoreOperationOutput) {
        let shell = self.shell.clone();
        let CruxRequest::Id(request_id) = request else {
            return;
        };

        let _ = shell.notify(MessageToShell::HandleResponse(*request_id, Box::new(response))).await;
    }

    async fn response_throttle(&self, request: &mut CruxRequest, response: CoreOperationOutput) {
        let CruxRequest::Id(request_id) = request else {
            return;
        };

        self.throttle_shell_runtime
            .send(MessageToShell::HandleResponse(*request_id, Box::new(response)))
            .await;
    }

    async fn notify(&self, event: AppEvent) {
        self.shell.request(MessageToShell::Notify(Box::new(event))).await;
    }
}
