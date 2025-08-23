use crate::executor::message_to_shell::MessageToShell;
use crate::{ShellRuntime, ThrottleShellRuntime};
use n0_future::task::{spawn, JoinHandle};
use shared::app::operations::CoreOperationOutput;
use shared::core_api::CoreBridge;
use std::sync::Arc;
use std::time::Duration;

pub struct CoreBridgeImpl {
    pub shell: Arc<ShellRuntime>,
    pub throttle_shell_runtime: ThrottleShellRuntime<MessageToShell>
}

impl CoreBridgeImpl {
    pub fn new(shell: Arc<ShellRuntime>) -> Self {
        Self {
            throttle_shell_runtime: ThrottleShellRuntime::new(shell.clone(), Duration::from_millis(100)),
            shell
        }
    }
}

#[async_trait::async_trait(?Send)]
impl CoreBridge for CoreBridgeImpl {
    fn response(&self, request_id: u32, response: CoreOperationOutput) -> JoinHandle<()> {
        let shell = self.shell.clone();
        spawn(async move {
            let _ = shell.notify(MessageToShell::HandleResponse(request_id, response)).await;
        })
    }

    async fn response_throttle(&self, request_id: u32, response: CoreOperationOutput) {
        self.throttle_shell_runtime.send(MessageToShell::HandleResponse(request_id, response)).await;
    }
}
