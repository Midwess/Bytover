use std::sync::{Arc, LazyLock};
use crux_core::Core;
use tauri::Manager;
use tokio::{fs, spawn};
use core_services::logger;
use native::di_container::DiContainer;
use shared::app::{AppEvent, AppOperation};
use shared::app::BitBridge;
use shared::app::environment::module::EnvironmentEvent;
use shared::app::operations::CoreOperationOutput;
use shared::app::operations::device::DeviceOperation;
use shared::app::operations::dialog::DialogOperation;
use shared::CoreOperation;
use shared::shell::api::{CoreRequest, CruxRequest};
use crate::api::bridge::BridgeImpl;
use crate::api::path_resolver::PathResolverImpl;

pub mod api;
static CORE: LazyLock<Arc<Core<BitBridge>>> = LazyLock::new(|| Arc::new(Core::new()));

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

async fn process_event(event: AppEvent) {
    let effects = CORE.process_event(event);
    process_effects(effects).await;
}

async fn process_effects(mut effects: Vec<AppOperation>) {
    while let Some(effect) = effects.pop() {
        let AppOperation::Operation(request) = effect;

        let (operation, mut handle) = request.split();
        let mut new_effects = match operation {
            CoreOperation::Notified(event) => {
                CORE.process_event(event)
            },
            CoreOperation::InitNativeExecutor => {
                CORE.resolve(&mut handle, CoreOperationOutput::None).unwrap_or_default()
            },
            CoreOperation::Device(device) => match device {
                DeviceOperation::GetDeviceInfo => {
                    CORE.resolve(&mut handle, CoreOperationOutput::None).unwrap_or_default()
                }
                DeviceOperation::GetGeoLocation => {
                    CORE.resolve(&mut handle, CoreOperationOutput::None).unwrap_or_default()
                }
                DeviceOperation::OpenSession(_) => {
                    CORE.resolve(&mut handle, CoreOperationOutput::None).unwrap_or_default()
                }
                DeviceOperation::Open(_) => {
                    CORE.resolve(&mut handle, CoreOperationOutput::None).unwrap_or_default()
                }
                DeviceOperation::LoadThumbnailPng(_) => {
                    CORE.resolve(&mut handle, CoreOperationOutput::None).unwrap_or_default()
                }
            },
            CoreOperation::Dialog(dialog) => match dialog {
                DialogOperation::Toast(_) => {
                    CORE.resolve(&mut handle, CoreOperationOutput::None).unwrap_or_default()
                }
                DialogOperation::Alert(_) => {
                    CORE.resolve(&mut handle, CoreOperationOutput::None).unwrap_or_default()
                }
                DialogOperation::Message(_, _) => {
                    CORE.resolve(&mut handle, CoreOperationOutput::None).unwrap_or_default()
                }
            }
            operation => {
                spawn(async move {
                    let bridge = DiContainer::get_instance().core_bridge();
                    let request = CoreRequest::new(CruxRequest::RequestHandle(handle), bridge);
                    let executor = DiContainer::get_instance().get_native_executor();
                    let output = executor.handle(request.clone(), operation).await;
                    request.response(output).await;
                });
                continue;
            }
        };

        effects.append(&mut new_effects);
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub async fn run() {
    logger::setup();
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
                process_event(EnvironmentEvent::AppLaunched.into()).await;
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
