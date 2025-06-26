// Only compile these modules when "lib" feature is enabled

static _CURRENT_VERSION: &str = "1.0.0";

#[cfg(feature = "lib")]
use tokio::sync::Mutex;
#[cfg(feature = "lib")]
use tokio::time::{self, Duration, Interval};
#[cfg(feature = "lib")]
pub mod config;
#[cfg(feature = "lib")]
pub mod di_container;
#[cfg(feature = "lib")]
pub mod errors;
#[cfg(feature = "lib")]
pub mod grpc;
#[cfg(feature = "lib")]
pub mod native;
#[cfg(feature = "lib")]
pub mod network;
#[cfg(feature = "lib")]
pub mod repository;
#[cfg(feature = "lib")]
use bincode::Options;
#[cfg(feature = "lib")]
pub use crux_core::{bridge::Bridge, Core, Request};
#[cfg(feature = "lib")]
use di_container::DiContainer;
#[cfg(feature = "lib")]
use erased_serde::Serialize;
#[cfg(feature = "lib")]
use lazy_static::lazy_static;
#[cfg(feature = "lib")]
use native::executor::NativeExecutor;
#[cfg(feature = "lib")]
use shared::app::operations::CoreOperation;
#[cfg(feature = "lib")]
use shared::app::BitBridge;
#[cfg(feature = "lib")]
use std::sync::Arc;
#[cfg(feature = "lib")]
use tokio::sync::OnceCell;
#[cfg(feature = "lib")]
use tokio::{spawn, task::JoinHandle};

#[cfg(feature = "lib")]
pub static TOKIO_RT: OnceCell<tokio::runtime::Runtime> = OnceCell::const_new();

#[cfg(feature = "lib")]
#[async_trait::async_trait]
pub trait ShellRuntime: Send + Sync + 'static {
    async fn msg_from_native(&self, event: Vec<u8>);
    fn msg_from_native_bg(self: Arc<Self>, event: Vec<u8>) -> JoinHandle<()> {
        let self_clone = self.clone();
        spawn(async move {
            self_clone.msg_from_native(event).await;
        })
    }
}

#[cfg(feature = "lib")]
pub struct ThrottleShellRuntime<E: Serialize + Send + 'static> {
    latest_event: Arc<Mutex<Option<E>>>,
    join_handle: JoinHandle<()>
}

#[cfg(feature = "lib")]
impl<E: Serialize + Send + Sync + 'static> ThrottleShellRuntime<E> {
    pub fn new(shell_runtime: Arc<dyn ShellRuntime>, delay: Duration) -> Self {
        let latest_event = Arc::new(Mutex::new(None::<E>));
        let latest_event_clone = latest_event.clone();
        let shell_runtime_clone = shell_runtime.clone();

        let join_handle = spawn(async move {
            let mut interval: Interval = time::interval(delay);
            interval.tick().await;

            loop {
                interval.tick().await;

                let event_to_send = {
                    let mut latest = latest_event_clone.lock().await;
                    latest.take()
                };

                if let Some(event) = event_to_send {
                    let serialized_event = serialize(&event);
                    shell_runtime_clone.clone().msg_from_native_bg(serialized_event);
                }
            }
        });

        Self { latest_event, join_handle }
    }

    pub async fn send(&self, event: E) {
        let mut latest = self.latest_event.lock().await;
        *latest = Some(event);
    }
}

#[cfg(feature = "lib")]
impl<E: Serialize + Send + 'static> Drop for ThrottleShellRuntime<E> {
    fn drop(&mut self) {
        let handle = self.join_handle.abort_handle();
        handle.abort();
    }
}

// NativeProcessor implementation
#[cfg(feature = "lib")]
pub struct NativeProcessor {
    shell: Arc<dyn ShellRuntime>,
    native_executor: Arc<NativeExecutor>
}

#[cfg(feature = "lib")]
pub fn get_tokio_rt() -> &'static tokio::runtime::Runtime {
    TOKIO_RT.get().unwrap_or_else(|| {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .thread_name("bitbridge-native-worker")
            .enable_all()
            .build()
            .unwrap();
        TOKIO_RT.set(rt).unwrap();
        TOKIO_RT.get().expect("Tokio runtime not initialized")
    })
}

#[cfg(feature = "lib")]
impl NativeProcessor {
    pub async fn new(shell: Arc<dyn ShellRuntime>, private_path: String, public_path: String) -> Self {
        use shared::app::file_system::workdir::WorkDir;

        let shell: Arc<dyn ShellRuntime> = shell;
        let di_container = DiContainer::get_instance();
        di_container.init(WorkDir::new(
            private_path,
            public_path
        )).await;
        let native_executor = Arc::new(di_container.get_native_executor());

        Self {
            shell: shell.clone(),
            native_executor,
        }
    }

    pub async fn handle(&self, id: u32, effect: Vec<u8>) -> Vec<u8> {
        let options = bincode_options();
        let mut deser = bincode::Deserializer::from_slice(&effect, options);
        let mut deserializer = <dyn erased_serde::Deserializer>::erase(&mut deser);

        let effect: CoreOperation = erased_serde::deserialize(&mut deserializer).expect("Failed to deserialize effect");

        let native_executor = self.native_executor.clone();
        let shell = self.shell.clone();
        get_tokio_rt()
            .spawn(async move {
                let output = native_executor.handle(id, effect, shell).await;
                handle_response(id, serialize(&output))
            })
            .await
            .unwrap_or_default()
    }
}

#[cfg(feature = "lib")]
uniffi::include_scaffolding!("shared");

#[cfg(feature = "lib")]
lazy_static! {
    pub static ref CORE_BRIDGE: Bridge<BitBridge> = Bridge::new(Core::new());
}

#[cfg(feature = "lib")]
pub fn process_event(msg: Vec<u8>) -> Vec<u8> {
    CORE_BRIDGE.process_event(&msg).unwrap_or_default()
}

#[cfg(feature = "lib")]
pub fn handle_response(id: u32, res: Vec<u8>) -> Vec<u8> {
    CORE_BRIDGE.handle_response(id, &res).unwrap_or_default()
}

#[cfg(feature = "lib")]
pub fn view() -> Vec<u8> {
    CORE_BRIDGE.view().unwrap_or_default()
}

#[cfg(feature = "lib")]
pub fn serialize<E: Serialize>(data: &E) -> Vec<u8> {
    let options = bincode_options();
    let mut buffer = Vec::new();
    let mut serializer = bincode::Serializer::new(&mut buffer, options);
    erased_serde::serialize(data, &mut serializer).unwrap();
    buffer
}

#[cfg(feature = "lib")]
fn bincode_options() -> impl bincode::Options + Copy {
    bincode::DefaultOptions::new().with_fixint_encoding().allow_trailing_bytes()
}
