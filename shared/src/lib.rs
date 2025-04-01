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
use std::sync::Arc;

#[cfg(feature = "lib")]
mod instrument;

#[cfg(feature = "lib")]
use app::operations::CoreOperationOutput;

#[cfg(feature = "lib")]
lazy_static! {
    pub static ref INSTRUMENT: Arc<Instrument> = Arc::new(Instrument::new());
}

#[cfg(feature = "lib")]
lazy_static! {
    pub static ref TOKIO_RT: tokio::runtime::Runtime =
        tokio::runtime::Builder::new_multi_thread().max_blocking_threads(10).enable_all().build().unwrap();
}

#[cfg(feature = "lib")]
pub trait ShellRuntime: Send + Sync {
    fn msg_from_native(&self, event: Vec<u8>);
}

// NativeProcessor implementation
#[cfg(feature = "lib")]
pub struct NativeProcessor {
    shell: Arc<dyn ShellRuntime>,
    native_executor: NativeExecutor
}

#[cfg(feature = "lib")]
impl NativeProcessor {
    pub fn new(shell: Box<dyn ShellRuntime>) -> Self {
        let shell: Arc<dyn ShellRuntime> = Arc::from(shell);
        let di_container = DiContainer::get_instance();
        let native_executor: NativeExecutor = di_container.get_native_executor(shell.clone());

        Self {
            shell: shell.clone(),
            native_executor
        }
    }

    pub fn handle(&self, id: u32, effect: &[u8]) -> Vec<u8> {
        let options = bincode_options();
        let mut deser = bincode::Deserializer::from_slice(effect, options);
        let mut deserializer = <dyn erased_serde::Deserializer>::erase(&mut deser);

        let effect: CoreOperation = erased_serde::deserialize(&mut deserializer).expect("Failed to deserialize effect");

        let mut output: Option<CoreOperationOutput> = None;
        TOKIO_RT.block_on(async {
            log::info!(target: "mem-usage", "Memory log: {:?}", INSTRUMENT.mem_log().await);
            output = Some(self.native_executor.handle(effect, self.shell.clone()).await);
        });

        match output {
            Some(output) => handle_response(id, &serialize(&output)),
            None => Vec::new()
        }
    }
}

#[cfg(feature = "lib")]
uniffi::include_scaffolding!("shared");

#[cfg(feature = "lib")]
lazy_static! {
    pub static ref CORE_BRIDGE: Bridge<BitBridge> = Bridge::new(Core::new());
}

#[cfg(feature = "lib")]
pub fn process_event(msg: &[u8]) -> Vec<u8> {
    CORE_BRIDGE.process_event(msg).unwrap_or_default()
}

#[cfg(feature = "lib")]
pub fn handle_response(id: u32, res: &[u8]) -> Vec<u8> {
    CORE_BRIDGE.handle_response(id, res).unwrap_or_default()
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
