use crate::{forward_core_operation_output, serialize, update_app_event};
use futures::lock::Mutex;
use n0_future::task::{spawn, JoinHandle};
use n0_future::time;
use n0_future::time::Interval;
use shared::app::operations::CoreOperationOutput;
use shared::app::AppEvent;
use shared::shell::api::{CoreBridge, CruxRequest};
use std::sync::Arc;
use std::time::Duration;

pub struct CoreBridgeImpl {
    pub shell: Arc<ShellRuntime>,
    pub throttle_shell_runtime: ThrottleShellRuntime,
}

impl CoreBridgeImpl {
    pub fn new() -> Self {
        let shell = Arc::new(ShellRuntime {});
        Self {
            throttle_shell_runtime: ThrottleShellRuntime::new(shell.clone(), Duration::from_millis(500)),
            shell,
        }
    }
}

#[async_trait::async_trait(?Send)]
impl CoreBridge for CoreBridgeImpl {
    async fn response(&self, request: &mut CruxRequest, response: CoreOperationOutput) {
        let shell = self.shell.clone();
        let CruxRequest::Id(request_id) = request else {
            panic!("Invalid request");
        };

        shell.forward_core_operation_output(*request_id, response).await;
    }

    async fn response_throttle(&self, request: &mut CruxRequest, response: CoreOperationOutput) {
        let CruxRequest::Id(request_id) = request else {
            panic!("Invalid request");
        };

        let _ = self.throttle_shell_runtime.send(*request_id, response).await;
    }

    async fn notify(&self, event: AppEvent) {
        let _ = self.shell.clone().update(event).await;
    }
}

pub struct ShellRuntime {}

impl ShellRuntime {
    async fn forward_core_operation_output(self: Arc<Self>, request_id: u32, output: CoreOperationOutput) {
        let serialized_output = serialize(&output);
        forward_core_operation_output(request_id, serialized_output).await;
    }

    fn update(self: Arc<Self>, event: AppEvent) -> JoinHandle<()> {
        spawn(async move {
            let serialized_event = serialize(&event);
            update_app_event(serialized_event).await;
        })
    }
}

pub struct ThrottleShellRuntime {
    latest_event: Arc<Mutex<Option<(u32, CoreOperationOutput)>>>,
}

impl ThrottleShellRuntime {
    pub fn new(shell_runtime: Arc<ShellRuntime>, delay: Duration) -> Self {
        let latest_event = Arc::new(Mutex::new(None::<(u32, CoreOperationOutput)>));
        let latest_event_clone = latest_event.clone();
        let shell_runtime_clone = shell_runtime.clone();

        spawn(async move {
            let mut interval: Interval = time::interval(delay);
            interval.tick().await;

            loop {
                interval.tick().await;

                let event_to_send = {
                    let mut latest = latest_event_clone.lock().await;
                    latest.take()
                };

                if let Some(event) = event_to_send {
                    let _ = shell_runtime_clone.clone().forward_core_operation_output(event.0, event.1).await;
                }
            }
        });

        Self { latest_event }
    }

    pub async fn send(&self, request_id: u32, event: CoreOperationOutput) {
        let mut latest = self.latest_event.lock().await;
        *latest = Some((request_id, event));
    }
}
