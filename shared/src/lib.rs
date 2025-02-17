// Only compile these modules when "lib" feature is enabled
#[cfg(feature = "lib")]
pub mod app;
#[cfg(feature = "lib")]
pub mod errors;
#[cfg(feature = "lib")]
pub mod native;
#[cfg(feature = "lib")]
pub mod persistence;
#[cfg(feature = "lib")]
pub mod di_container;
#[cfg(feature = "lib")]
pub mod config;
#[cfg(feature = "lib")]
pub mod grpc;
#[cfg(feature = "lib")]
pub mod entities;

#[cfg(feature = "lib")]
use app::{operations::{CoreOperation}, BitBridge};
#[cfg(feature = "lib")]
use bincode::Options;
#[cfg(feature = "lib")]
use di_container::DiContainer;
#[cfg(feature = "lib")]
use erased_serde::Serialize;
#[cfg(feature = "lib")]
use lazy_static::lazy_static;
#[cfg(feature = "lib")]
use tokio_scoped::scoped;
#[cfg(feature = "lib")]
use wasm_bindgen::prelude::wasm_bindgen;

#[cfg(feature = "lib")]
pub use crux_core::{bridge::Bridge, Core, Request};

#[cfg(feature = "lib")]
uniffi::include_scaffolding!("shared");

#[cfg(feature = "lib")]
lazy_static! {
    pub static ref CORE_BRIDGE: Bridge<BitBridge> = Bridge::new(Core::new());
}

// Only used tokio runtime when using tokio feature, otherwise it might impact on CRUX
// Detailed here: https://redbadger.github.io/crux/internals/runtime.html#admonition-warning
#[cfg(feature = "lib")]
lazy_static! {
    pub static ref TOKIO_RT: tokio::runtime::Runtime = tokio::runtime::Runtime::new().unwrap();
}

#[cfg(feature = "lib")]
#[wasm_bindgen]
pub fn process_event(data: &[u8]) -> Vec<u8> {
    CORE_BRIDGE.process_event(data)
}

#[cfg(feature = "lib")]
#[wasm_bindgen]
pub fn handle_response(id: u32, data: &[u8]) -> Vec<u8> {
    CORE_BRIDGE.handle_response(id, data)
}

#[cfg(feature = "lib")]
#[wasm_bindgen]
pub fn view() -> Vec<u8> {
    CORE_BRIDGE.view()
}

#[cfg(feature = "lib")]
#[wasm_bindgen]
pub fn native_handle(id: u32, data: &[u8]) -> Vec<u8> {
    let options = bincode_options();
    let mut deser = bincode::Deserializer::from_slice(data, options);
    let mut deserializer = <dyn erased_serde::Deserializer>::erase(&mut deser);

    let effect: CoreOperation = erased_serde::deserialize(&mut deserializer)
        .expect("Failed to deserialize effect");

    let mut output_buffer = Vec::new();
    scoped(TOKIO_RT.handle()).scope(|scope| {
        scope.spawn(async {
            let di_container = DiContainer::get_instance();
            let executor = di_container.get_native_executor().await;
            let output = executor.handle(effect).await;
            let response = serialize(&output);

            output_buffer = handle_response(id, &response);
        });
    });

    output_buffer
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
    bincode::DefaultOptions::new()
        .with_fixint_encoding()
        .allow_trailing_bytes()
}
