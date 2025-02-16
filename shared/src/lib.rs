pub mod app;
pub mod errors;
pub mod native;
pub mod persistence;
pub mod di_container;
pub mod config;
pub mod grpc;

use app::{operations::{CoreOperation, CoreOperationOutput}, AppEffect, AppEvent, BitBridge, BitBridgeEffect};
use bincode::{DefaultOptions, Options};
use di_container::DiContainer;
use erased_serde::Serialize;
use lazy_static::lazy_static;
use tokio_scoped::scoped;
use wasm_bindgen::prelude::wasm_bindgen;

pub use crux_core::{bridge::Bridge, Core, Request};

uniffi::include_scaffolding!("shared");

lazy_static! {
    pub static ref CORE_BRIDGE: Bridge<BitBridge> = Bridge::new(Core::new());
}

// Only used tokio runtime when using tokio feature, otherwise it might impact on CRUX
// Detailed here: https://redbadger.github.io/crux/internals/runtime.html#admonition-warning
lazy_static! {
    pub static ref TOKIO_RT: tokio::runtime::Runtime = tokio::runtime::Runtime::new().unwrap();
}

#[wasm_bindgen]
pub fn process_event(data: &[u8]) -> Vec<u8> {
    CORE_BRIDGE.process_event(data)
}

#[wasm_bindgen]
pub fn handle_response(id: u32, data: &[u8]) -> Vec<u8> {
    CORE_BRIDGE.handle_response(id, data)
}

#[wasm_bindgen]
pub fn view() -> Vec<u8> {
    CORE_BRIDGE.view()
}

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

pub fn serialize<E: Serialize>(data: &E) -> Vec<u8> {
    let options = bincode_options();
    let mut buffer = Vec::new();
    let mut serializer = bincode::Serializer::new(&mut buffer, options);
    erased_serde::serialize(data, &mut serializer).unwrap();
    buffer
}

// Match the same bincode options used in Bridge
fn bincode_options() -> impl bincode::Options + Copy {
    bincode::DefaultOptions::new()
        .with_fixint_encoding()
        .allow_trailing_bytes()
}
