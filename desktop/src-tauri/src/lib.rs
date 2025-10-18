use std::ops::Deref;
use std::sync::{Arc, LazyLock};
use crux_core::Core;
use tauri::Manager;
use tokio::{fs, spawn};
use native::di_container::DiContainer;
use shared::app::AppEvent;
use shared::app::BitBridge;
use shared::shell::api::CoreBridge;
use crate::api::bridge::BridgeImpl;
use crate::api::path_resolver::PathResolverImpl;

pub mod api;
static CORE: LazyLock<Arc<Core<BitBridge>>> = LazyLock::new(|| Arc::new(Core::new()));

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn process(event: AppEvent) {}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub async fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet])
        .setup(|app| {
            let workdir_path = app
                .path()
                .app_data_dir()
                .expect("We still solving issue that don't have app data dir");

            spawn(async move {
                let _ = fs::create_dir_all(&workdir_path);
                let bridge = Box::leak(Box::new(BridgeImpl {}));
                DiContainer::get_instance().init(Arc::new(PathResolverImpl::new(workdir_path).await), &*bridge).await;
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
