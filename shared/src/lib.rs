pub mod app;
pub mod native;
pub mod persistence;
pub mod di_container;
pub mod config;
pub mod grpc;

use app::{operations::{CoreOperation, CoreOperationOutput}, AppEffect, AppEvent, BitBridge, BitBridgeEffect};
use bincode::{DefaultOptions, Options};
use erased_serde::Serialize;
use lazy_static::lazy_static;
use native::executor::NativeExecutor;
use tokio_scoped::scoped;
use wasm_bindgen::prelude::wasm_bindgen;

pub use crux_core::{bridge::Bridge, Core, Request};

uniffi::include_scaffolding!("shared");

lazy_static! {
    pub static ref CORE_BRIDGE: Bridge<BitBridge> = Bridge::new(Core::new());
}

lazy_static! {
    pub static ref NATIVE_EXECUTOR: NativeExecutor = NativeExecutor {};
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
            let output = NATIVE_EXECUTOR.handle(effect).await;
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

#[cfg(test)]
mod test {
    use std::process::Output;

    use core_services::logger;
    use lazy_static::lazy_static;
    use crux_core::{Core};
    use schema::{devlog::bitbirdge::effects, value::platform::Platform};
    use crate::{app::{modules::{authentication::AuthenticationEvent, environment::DeviceInfo}, operations::{device::DeviceOperationOutput, CoreOperation, CoreOperationOutput}, AppEvent, BitBridge, BitBridgeEffect}, di_container::DiContainer};

    lazy_static! {
        static ref CORE: Core<BitBridge> = Core::new();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn test_1() {
        logger::setup();
        DiContainer::get_instance().init("~/.test".to_owned()).await;
        let mut effects = CORE.process_event(AppEvent::Authentication(AuthenticationEvent::SignIn));
        while let Some(effect) = match effects.is_empty() {
            true => None,
            false => Some(effects.remove(0))
        } {
            match effect {
                BitBridgeEffect::AppCapabilities(mut request) => {
                    log::info!("Request found: {:?}", request);
                    let new_effects = CORE.resolve(&mut request, CoreOperationOutput::Device(DeviceOperationOutput::DeviceInfo(DeviceInfo {
                        name: "test".to_owned(),
                        platform: Platform::IOs,
                        unique_id: "test".to_owned(),
                    })));

                    effects.extend(new_effects.into_iter());
                }
            }
        }
    }
}
