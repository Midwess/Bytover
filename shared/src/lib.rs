pub mod app;
pub mod system;
pub mod persistence;
pub mod di_container;

use app::{modules::counter::CounterViewModel, BitBridge};
use bincode::{DefaultOptions, Options};
use erased_serde::Serialize;
use lazy_static::lazy_static;
use wasm_bindgen::prelude::wasm_bindgen;

pub use crux_core::{bridge::Bridge, Core, Request};

uniffi::include_scaffolding!("shared");

lazy_static! {
    pub static ref CORE: Bridge<BitBridge> = Bridge::new(Core::new());
}

#[wasm_bindgen]
pub fn process_event(data: &[u8]) -> Vec<u8> {
    CORE.process_event(data)
}

#[wasm_bindgen]
pub fn handle_response(id: u32, data: &[u8]) -> Vec<u8> {
    CORE.handle_response(id, data)
}

#[wasm_bindgen]
pub fn view() -> Vec<u8> {
    CORE.view()
}
