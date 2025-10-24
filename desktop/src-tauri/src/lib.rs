use hostname;
use machine_uid;
use std::sync::{Arc, LazyLock};
use crux_core::Core;
use schema::value::platform::Platform;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_opener::OpenerExt;
use tokio::{fs, spawn};
use core_services::logger;
use native::di_container::DiContainer;
use shared::app::{AppEvent, AppOperation, AppViewModel};
use shared::app::authentication::module::AuthenticationEvent;
use shared::app::BitBridge;
use shared::app::environment::module::EnvironmentEvent;
use shared::app::operations::CoreOperationOutput;
use shared::app::operations::device::DeviceOperation;
use shared::app::operations::dialog::DialogOperation;
use shared::app::operations::webview::WebViewOperation;
use shared::CoreOperation;
use shared::entities::device::DeviceInfo;
use shared::shell::api::{CoreRequest, CruxRequest};
use uuid::Uuid;
use schema::value::device::DeviceType;
use crate::api::bridge::BridgeImpl;
use crate::api::path_resolver::PathResolverImpl;

pub mod api;
static CORE: LazyLock<Arc<Core<BitBridge>>> = LazyLock::new(|| Arc::new(Core::new()));

#[tauri::command]
async fn sign_in(app_handle: AppHandle) {
    process_event(AppEvent::Authentication(AuthenticationEvent::SignIn), app_handle).await;
}

async fn process_event(event: AppEvent, app_handle: AppHandle) {
    let effects = CORE.process_event(event);
    process_effects(effects, app_handle).await;
}

fn render(view: AppViewModel, app_handle: AppHandle) {
    let is_authorized = view.authentication.as_ref().map(|auth| auth.user.is_some()).unwrap_or(false);
    if !is_authorized {
        for (_, window) in app_handle.webview_windows() {
            if window.label() != "auth" {
                let _ = window.close();
            }
        }

        let auth_window = match app_handle.get_webview_window("auth") {
            Some(window) => window,
            None => {
                let win = tauri::WebviewWindowBuilder::new(
                    &app_handle,
                    "auth",
                    tauri::WebviewUrl::App("auth.html".into())
                )
                .title("Auth")
                .build().unwrap();
                win
            }
        };

        auth_window.show().unwrap();
        auth_window.set_focus().unwrap();
    }

    let _ = app_handle.emit("Render", view);
}

async fn process_effects(mut effects: Vec<AppOperation>, app_handle: AppHandle) {
    while let Some(effect) = effects.pop() {
        let AppOperation::Operation(request) = effect;

        let (operation, mut handle) = request.split();
        let mut new_effects = match operation {
            CoreOperation::Render => {
                render(CORE.view(), app_handle.clone());
                CORE.resolve(&mut handle, CoreOperationOutput::None).unwrap_or_default()
            }
            CoreOperation::Notified(event) => {
                CORE.process_event(event)
            },
            CoreOperation::InitNativeExecutor => {
                CORE.resolve(&mut handle, CoreOperationOutput::None).unwrap_or_default()
            },
            CoreOperation::Device(device) => match device {
                DeviceOperation::GetDeviceInfo => {
                    let platform = if cfg!(target_os = "windows") { Platform::Windows }
                    else if cfg!(target_os = "macos") { Platform::MacOs }
                    else { Platform::Linux };

                    let device_type = if cfg!(target_os = "macos") { DeviceType::Macbook }
                    else { DeviceType::OtherLaptop };

                    let device = DeviceInfo {
                        platform,
                        name: hostname::get().ok().and_then(|it| it.to_str().map(|it| it.to_owned())).unwrap_or(Uuid::new_v4().to_string()),
                        device_type,
                        unique_id: machine_uid::get().unwrap_or(Uuid::new_v4().to_string())
                    };

                    log::info!("device info: {:?}", device);
                    CORE.resolve(&mut handle, CoreOperationOutput::DeviceInfo(device)).unwrap_or_default()
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
            CoreOperation::WebView(WebViewOperation::OpenUrl(url)) => {
                let _ = app_handle.opener().open_url(url, Option::<&str>::None);
                CORE.resolve(&mut handle, CoreOperationOutput::None).unwrap_or_default()
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
        .invoke_handler(tauri::generate_handler![sign_in])
        .setup(|app| {
            let handle = app.handle().clone();
            let workdir_path = app
                .path()
                .app_data_dir()
                .expect("We still solving issue that don't have app data dir");

            spawn(async move {
                let _ = fs::create_dir_all(&workdir_path);
                let bridge = Box::leak(Box::new(BridgeImpl {
                    app_handle: handle.clone()
                }));

                DiContainer::get_instance().init(Arc::new(PathResolverImpl::new(workdir_path).await), &*bridge).await;
                process_event(EnvironmentEvent::AppLaunched.into(), handle.clone()).await;
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
