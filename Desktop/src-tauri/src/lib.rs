use std::sync::Arc;

use core_services::logger;
use shared::{app::{Counter, Event}, Core};
use lazy_static::lazy_static;

lazy_static! {
    static ref CORE: Arc<Core<Counter>> = Arc::new(Core::new());
}

#[tauri::command]
fn increment() {
    log::info!(target: "tiendang-debug", "Incrementing");
    CORE.process_event(Event::Increment);
}

#[tauri::command]
fn decrement() {
    log::info!(target: "tiendang-debug", "Decrementing");
    CORE.process_event(Event::Decrement);
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
