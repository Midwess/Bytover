use std::sync::Arc;

use core_services::logger;
use shared::{Core, app::BitBridge};
use lazy_static::lazy_static;

lazy_static! {
    static ref CORE: Arc<Core<BitBridge>> = Arc::new(Core::new());
}

#[tauri::command]
fn increment() {
    log::info!(target: "tiendang-debug", "Incrementing");
}

#[tauri::command]
fn decrement() {
    log::info!(target: "tiendang-debug", "Decrementing");
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    logger::setup();
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![increment, decrement])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
