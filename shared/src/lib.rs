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
use instrument::Instrument;
#[cfg(feature = "lib")]
use lazy_static::lazy_static;
#[cfg(feature = "lib")]
use native::executor::NativeExecutor;
#[cfg(feature = "lib")]
use tokio::task::spawn_blocking;
#[cfg(feature = "lib")]
use tokio::{spawn, task::JoinHandle};

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
            let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build().unwrap();
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
            return get_tokio_rt().spawn_blocking(move || {
                handle_response(id, serialize(&out))
            }).await.unwrap_or_default()
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
