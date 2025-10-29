use std::env::var;
use std::path::PathBuf;
use crate::api::bridge::BridgeImpl;
use crate::api::path_resolver::PathResolverImpl;
use core_services::logger;
use crux_core::Core;
use native::di_container::DiContainer;
use schema::value::device::DeviceType;
use schema::value::platform::Platform;
use shared::app::authentication::module::AuthenticationEvent;
use shared::app::environment::module::EnvironmentEvent;
use shared::app::operations::device::DeviceOperation;
use shared::app::operations::dialog::DialogOperation;
use shared::app::operations::webview::WebViewOperation;
use shared::app::operations::CoreOperationOutput;
use shared::app::{AppEvent, AppOperation, AppViewModel, BitBridge};
use shared::entities::device::DeviceInfo;
use shared::shell::api::{CoreRequest, CruxRequest};
use shared::CoreOperation;
use std::sync::{Arc, LazyLock};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_deep_link::DeepLinkExt;
use tauri_plugin_opener::OpenerExt;
use tokio::{fs, spawn};
use uuid::Uuid;
use {hostname, machine_uid};
use shared::app::shelf::module::{ResourceSelection, ShelfEvent};
use shared::app::transfer::module::TransferEvent;
use shared::entities::local_resource::LocalResourcePath;
use crate::extensions::AppHandleExt;
use crate::thumbnail::generate_thumbnail;

pub mod api;
pub mod extensions;
mod thumbnail;

static CORE: LazyLock<Arc<Core<BitBridge>>> = LazyLock::new(|| Arc::new(Core::new()));

#[tauri::command]
async fn start_dragging(
    app_handle: AppHandle,
    file_path: String,
    thumbnail_path: Option<String>
) {
    log::info!("Starting drag for file: {}", file_path);
    let Some(window) = app_handle.get_focused_window() else {
        log::warn!("No window is focused, cannot start dragging");
        return;
    };

    let result = handle_drag(&window, file_path, thumbnail_path);
    log::info!("Drag result: {:?}", result);
}

fn handle_drag(
    window: &tauri::Window,
    file_path: String,
    thumbnail_path: Option<String>
) -> Result<(), Box<dyn std::error::Error>> {
    // Validate and canonicalize the file path
    let abs_path = std::fs::canonicalize(&file_path)?;
    log::info!("Starting drag for file: {}", abs_path.display());

    // Ensure the file exists
    if !abs_path.exists() {
        return Err(format!("File not found: {}", file_path).into());
    }

    let item = drag::DragItem::Files(vec![abs_path]);

    // Use provided thumbnail or fallback to a default preview
    let preview = if let Some(thumb_path) = thumbnail_path {
        // Validate thumbnail exists before using it
        let thumb_abs = std::fs::canonicalize(&thumb_path)
            .unwrap_or_else(|_| {
                log::warn!("Thumbnail not found, using fallback: {}", thumb_path);
                PathBuf::from("./default-icon.png")
            });

        if thumb_abs.exists() {
            drag::Image::File(thumb_abs)
        } else {
            log::warn!("Thumbnail file doesn't exist: {:?}, using fallback", thumb_abs);
            drag::Image::Raw(include_bytes!("../../public/send.svg").to_vec())
        }
    } else {
        // Use embedded default preview
        drag::Image::Raw(include_bytes!("../../public/send.svg").to_vec())
    };

    // Get the window handle with proper platform-specific handling
    #[cfg(target_os = "linux")]
    {
        let gtk_window = window.gtk_window()?;
        drag::start_drag(
            &gtk_window,
            item,
            preview,
            |result, cursor| {
                match result {
                    drag::DragResult::Dropped => {
                        log::info!("File dropped at x:{}, y:{}", cursor.x, cursor.y);
                    }
                    drag::DragResult::Cancel => {
                        log::info!("Drag operation cancelled");
                    }
                }
            },
            drag::Options::default(),
        )?;
    }

    #[cfg(target_os = "macos")]
    {
        drag::start_drag(
            window,
            item,
            preview,
            |result, cursor| {
                match result {
                    drag::DragResult::Dropped => {
                        log::info!("File dropped at x:{}, y:{}", cursor.x, cursor.y);
                    }
                    drag::DragResult::Cancel => {
                        log::info!("Drag operation cancelled");
                    }
                }
            },
            drag::Options::default(),
        )?;
    }

    #[cfg(target_os = "windows")]
    {
        drag::start_drag(
            window,
            item,
            preview,
            |result, cursor| {
                match result {
                    drag::DragResult::Dropped => {
                        log::info!("File dropped at x:{}, y:{}", cursor.x, cursor.y);
                    }
                    drag::DragResult::Cancel => {
                        log::info!("Drag operation cancelled");
                    }
                }
            },
            drag::Options::default(),
        )?;
    }

    Ok(())
}

#[tauri::command]
async fn ui_launched(app_handle: AppHandle) {
    render(CORE.view(), app_handle);
}

#[tauri::command]
async fn remove_resource(resource_id: String, app_handle: AppHandle) {
    let resource_id = resource_id.parse::<u64>().unwrap_or_default();
    process_event(ShelfEvent::RemoveResource(resource_id), app_handle).await;
}

#[tauri::command]
async fn sign_in(app_handle: AppHandle) {
    process_event(AuthenticationEvent::SignIn, app_handle).await;
}

#[tauri::command]
async fn start_transfer(target_id: String, app_handle: AppHandle) {
    process_event(TransferEvent::StartTransfer {
        target_id
    }, app_handle).await;
}

#[tauri::command]
async fn add_resources(paths: Vec<String>, app_handle: AppHandle) {
    let selections = paths.into_iter().map(|path| ResourceSelection {
        path: LocalResourcePath::AbsolutePath(path),
        r#type: None
    }).collect::<Vec<_>>();

    process_event(ShelfEvent::AddResources(selections), app_handle).await;
}

async fn process_event(event: impl Into<AppEvent> + Send + Sync + 'static, app_handle: AppHandle) {
    let effects = CORE.process_event(event.into());
    process_effects(effects, app_handle).await;
}

fn render(view: AppViewModel, app_handle: AppHandle) {
    let is_authorized = view.authentication.as_ref().map(|auth| auth.user.is_some()).unwrap_or(false);
    if !is_authorized {
        app_handle.show_auth();
    }
    else {
        app_handle.show_send();
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
                spawn(async move {
                    let bridge = DiContainer::get_instance().core_bridge();
                    bridge.notify(event).await;
                });

                CORE.resolve(&mut handle, CoreOperationOutput::None).unwrap_or_default()
            },
            CoreOperation::InitNativeExecutor => CORE.resolve(&mut handle, CoreOperationOutput::None).unwrap_or_default(),
            CoreOperation::Device(device) => match device {
                DeviceOperation::GetDeviceInfo => {
                    let platform = if cfg!(target_os = "windows") {
                        Platform::Windows
                    } else if cfg!(target_os = "macos") {
                        Platform::MacOs
                    } else {
                        Platform::Linux
                    };

                    let device_type = if cfg!(target_os = "macos") {
                        DeviceType::Macbook
                    } else {
                        DeviceType::OtherLaptop
                    };

                    let device = DeviceInfo {
                        platform,
                        name: hostname::get()
                            .ok()
                            .and_then(|it| it.to_str().map(|it| it.to_owned()))
                            .unwrap_or(Uuid::new_v4().to_string()),
                        device_type,
                        unique_id: machine_uid::get().unwrap_or(Uuid::new_v4().to_string())
                    };

                    CORE.resolve(&mut handle, CoreOperationOutput::DeviceInfo(device)).unwrap_or_default()
                }
                DeviceOperation::GetGeoLocation => CORE.resolve(&mut handle, CoreOperationOutput::None).unwrap_or_default(),
                DeviceOperation::OpenSession(_) => CORE.resolve(&mut handle, CoreOperationOutput::None).unwrap_or_default(),
                DeviceOperation::Open(_) => CORE.resolve(&mut handle, CoreOperationOutput::None).unwrap_or_default(),
                DeviceOperation::LoadThumbnailPng { path, resource_type, id } => match path {
                    LocalResourcePath::AbsolutePath(path) => {
                        let path = PathBuf::from(path);
                        let path_resolver = DiContainer::get_instance().path_resolver();
                        let output_path_str = path_resolver.get_thumbnail_file_path(id).await;
                        let output_path = PathBuf::from(&output_path_str);
                        if let Err(e) = generate_thumbnail(path.clone(), output_path, &resource_type).await {
                            log::error!("Failed to generate thumbnail for {path:?} {e:?}");
                            CORE.resolve(&mut handle, CoreOperationOutput::None).unwrap_or_default()
                        }
                        else {
                            CORE.resolve(&mut handle, CoreOperationOutput::LocalResourcePath(LocalResourcePath::AbsolutePath(output_path_str))).unwrap_or_default()
                        }
                    },
                    path => {
                        log::warn!("Desktop only support absolute path not {path:?}");
                        CORE.resolve(&mut handle, CoreOperationOutput::None).unwrap_or_default()
                    }
                }
            },
            CoreOperation::WebView(WebViewOperation::OpenUrl(url)) => {
                let _ = app_handle.opener().open_url(url, Option::<&str>::None);
                CORE.resolve(&mut handle, CoreOperationOutput::None).unwrap_or_default()
            }
            CoreOperation::Dialog(dialog) => match dialog {
                DialogOperation::Toast(msg) => {
                    log::info!(target: "toast", "{msg:?}");
                    CORE.resolve(&mut handle, CoreOperationOutput::None).unwrap_or_default()
                },
                DialogOperation::Alert(alert) => {
                    log::info!(target: "alert", "{alert:?}");
                    CORE.resolve(&mut handle, CoreOperationOutput::None).unwrap_or_default()
                },
                DialogOperation::Message(msg, reason) => {
                    log::info!(target: "msg", "{msg:?} {reason:?}");
                    CORE.resolve(&mut handle, CoreOperationOutput::None).unwrap_or_default()
                }
            },
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
    let mut builder = tauri::Builder::default().plugin(tauri_plugin_deep_link::init());

    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|_app, argv, _cwd| {
            log::info!("a new app instance was opened with {argv:?} and the deep link event was already triggered");
        }));
    }

    builder
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_drag::init())
        .invoke_handler(tauri::generate_handler![sign_in, start_transfer, add_resources, remove_resource, ui_launched, start_dragging])
        .setup(|app| {
            let handle = app.handle().clone();
            let workdir_path = app.path().app_data_dir().expect("We still solving issue that don't have app data dir");

            let access_url = var("BITBRIDGE_ACCESS_TOKEN").ok()
                .map(|it| format!("bitbridge://authorize?access_token={it}"));

            let mut start_urls = app.deep_link().get_current()?.unwrap_or_default();
            if let Some(mock_url) = access_url {
                start_urls.push(mock_url.parse().unwrap());
            };

            app.deep_link().on_open_url({
                let handle = handle.clone();
                move |event| {
                    if let Some(url) = event.urls().first().cloned() {
                        let handle = handle.clone();
                        spawn(async move {
                            process_event(AuthenticationEvent::OnRedirected { url: url.to_string() }, handle).await;
                        });
                    }
                }
            });

            spawn(async move {
                let _ = fs::create_dir_all(&workdir_path);
                let bridge = Box::leak(Box::new(BridgeImpl {
                    app_handle: handle.clone()
                }));

                DiContainer::get_instance()
                    .init(Arc::new(PathResolverImpl::new(workdir_path).await), &*bridge)
                    .await;
                process_event(EnvironmentEvent::AppLaunched, handle.clone()).await;

                if let Some(url) = start_urls.first().cloned() {
                    let handle = handle.clone();
                    log::info!("Received redirect url: {}", url);
                    process_event(AuthenticationEvent::OnRedirected { url: url.to_string() }, handle).await;
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
