// Only compile these modules when "lib" feature is enabled

static CURRENT_VERSION: &str = "1.0.0";

#[cfg(feature = "lib")]
pub mod app;
#[cfg(feature = "lib")]
pub mod config;
#[cfg(feature = "lib")]
pub mod di_container;
#[cfg(feature = "lib")]
pub mod entities;
#[cfg(feature = "lib")]
pub mod errors;
#[cfg(feature = "lib")]
pub mod grpc;
#[cfg(feature = "lib")]
pub mod native;
#[cfg(feature = "lib")]
pub mod network;
#[cfg(feature = "lib")]
pub mod persistence;

#[cfg(feature = "lib")]
use app::{operations::CoreOperation, BitBridge};
#[cfg(feature = "lib")]
use bincode::Options;
#[cfg(feature = "lib")]
pub use crux_core::{bridge::Bridge, Core, Request};
#[cfg(feature = "lib")]
use di_container::DiContainer;
#[cfg(feature = "lib")]
use erased_serde::Serialize;
#[cfg(feature = "lib")]
use futures_util::SinkExt;
#[cfg(feature = "lib")]
use futures_util::StreamExt;
#[cfg(feature = "lib")]
use instrument::Instrument;
#[cfg(feature = "lib")]
use lazy_static::lazy_static;
#[cfg(feature = "lib")]
use native::executor::NativeExecutor;
#[cfg(feature = "lib")]
use tokio::{spawn, task::JoinHandle};

use std::ops::{Deref, DerefMut};
#[cfg(feature = "lib")]
use std::sync::Arc;

#[cfg(feature = "lib")]
mod instrument;

#[cfg(feature = "lib")]
lazy_static! {
    pub static ref INSTRUMENT: Arc<Instrument> = Arc::new(Instrument::new());
}

#[cfg(feature = "lib")]
use tokio::sync::OnceCell;

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
pub struct DebouncedShellRuntime<E: Serialize + Send + 'static> {
    join_handle: Option<JoinHandle<()>>,
    sender: Option<futures_channel::mpsc::Sender<E>>
}

#[cfg(feature = "lib")]
impl<E: Serialize + Send + 'static> Deref for DebouncedShellRuntime<E> {
    type Target = futures_channel::mpsc::Sender<E>;

    fn deref(&self) -> &Self::Target {
        self.sender.as_ref().expect("Channel already closed")
    }
}

#[cfg(feature = "lib")]
impl<E: Serialize + Send + 'static> DerefMut for DebouncedShellRuntime<E> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.sender.as_mut().expect("Channel already closed")
    }
}

#[cfg(feature = "lib")]
impl<E: Serialize + Send + 'static> Drop for DebouncedShellRuntime<E> {
    fn drop(&mut self) {
        let sender = self.sender.take();
        let join_handle = self.join_handle.take();
        spawn(async move {
            if let Some(mut sender) = sender {
                let _ = sender.close().await;
            }

            if let Some(join_handle) = join_handle {
                join_handle.abort();
            }
        });
    }
}

#[cfg(feature = "lib")]
// Create a debounced message handler
// You will need to handle the aborting of the debounced stream
fn debounced_msg_stream<E: Serialize + Send + 'static>(
    shell_runtime: &Arc<dyn ShellRuntime>,
    delay: std::time::Duration
) -> DebouncedShellRuntime<E> {
    let shell_runtime = shell_runtime.clone();
    let (tx, rx) = futures_channel::mpsc::channel::<E>(1024);
    let mut debounced = debounced::debounced(rx, delay);
    let handle = spawn(async move {
        while let Some(event) = debounced.next().await {
            shell_runtime.msg_from_native(serialize(&event)).await;
        }
    });

    DebouncedShellRuntime {
        join_handle: Some(handle),
        sender: Some(tx)
    }
}

#[cfg(feature = "lib")]
use tokio::sync::Mutex;
#[cfg(feature = "lib")]
use tokio::time::{self, Duration, Interval};

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
    native_executor: NativeExecutor
}

#[cfg(feature = "lib")]
pub fn get_tokio_rt() -> &'static tokio::runtime::Runtime {
    match TOKIO_RT.get() {
        Some(rt) => rt,
        None => {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .thread_name("bitbridge-worker")
                .enable_all()
                .build()
                .unwrap();
            TOKIO_RT.set(rt).unwrap();
            TOKIO_RT.get().expect("Tokio runtime not initialized")
        }
    }
}

#[cfg(feature = "lib")]
impl NativeProcessor {
    pub fn new(shell: Arc<dyn ShellRuntime>) -> Self {
        let shell: Arc<dyn ShellRuntime> = shell;
        let di_container = DiContainer::get_instance();
        let native_executor: NativeExecutor = di_container.get_native_executor(shell.clone());

        Self {
            shell: shell.clone(),
            native_executor
        }
    }

    pub async fn handle(&self, id: u32, effect: Vec<u8>) -> Vec<u8> {
        let options = bincode_options();
        let mut deser = bincode::Deserializer::from_slice(&effect, options);
        let mut deserializer = <dyn erased_serde::Deserializer>::erase(&mut deser);

        let effect: CoreOperation = erased_serde::deserialize(&mut deserializer).expect("Failed to deserialize effect");

        let mut output = None;
        tokio_scoped::scoped(get_tokio_rt().handle()).scope(|scope| {
            scope.spawn(async {
                output = Some(self.native_executor.handle(id, effect, self.shell.clone()).await);
            });
        });

        if let Some(out) = output {
            return get_tokio_rt()
                .spawn_blocking(move || handle_response(id, serialize(&out)))
                .await
                .unwrap_or_default()
        }

        vec![]
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
