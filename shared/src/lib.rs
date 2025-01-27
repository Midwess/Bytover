pub mod app;
pub mod system;
pub mod persistence;
pub mod di_container;

use app::BitBridge;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::wasm_bindgen;

pub use crux_core::{bridge::Bridge, Core, Request};

lazy_static! {
    pub static ref CORE: Bridge<BitBridge> = Bridge::new(Core::new());
}

#[derive(Debug, Clone)]
#[wasm_bindgen]
pub enum AppModule {
    Counter,
    System
}

#[wasm_bindgen]
pub fn process_event(module: AppModule, data: &[u8]) -> Vec<u8> {
    CORE.process_event(data)
}

#[wasm_bindgen]
pub fn handle_response(module: AppModule, id: u32, data: &[u8]) -> Vec<u8> {
    CORE.handle_response(id, data)
}

#[wasm_bindgen]
pub fn view(module: AppModule) -> Vec<u8> {
    CORE.view()
}

uniffi::include_scaffolding!("shared");
