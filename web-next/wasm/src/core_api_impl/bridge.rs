use crate::{ShellRuntime, ThrottleShellRuntime};
use n0_future::task::{spawn, JoinHandle};
use shared::app::operations::CoreOperationOutput;
use shared::app::AppEvent;
use shared::core_api::CoreBridge;
use std::sync::Arc;
use std::time::Duration;

pub struct CoreBridgeImpl {
    pub shell: Arc<ShellRuntime>,
    pub throttle_shell_runtime: ThrottleShellRuntime
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
            let _ = shell.forward_core_operation_output(request_id, response);
        })
    }

    async fn response_throttle(&self, request_id: u32, response: CoreOperationOutput) {
        let _ = self.throttle_shell_runtime.send(request_id, response).await;
    }

    async fn notify(&self, event: AppEvent) {
        let _ = self.shell.clone().update(event);
    }
}
