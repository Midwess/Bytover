use std::env::var;
use std::path::PathBuf;
use std::sync::{Arc, LazyLock, Mutex};
use crate::api::bridge::BridgeImpl;
use crate::api::path_resolver::PathResolverImpl;
use core_services::logger;
use tauri_plugin_autostart::ManagerExt;
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
use shared::app::shelf::module::ShelfItemViewModel;
use shared::app::{AppEvent, AppOperation, AppViewModel, BitBridge};
use shared::entities::device::DeviceInfo;
use shared::shell::api::{CoreRequest, CruxRequest};
use shared::CoreOperation;
use tauri::{AppHandle, Emitter, Manager};
use tauri::menu::{MenuBuilder, MenuItemBuilder, SubmenuBuilder};
use tauri::tray::{TrayIcon, TrayIconBuilder};
use tauri_plugin_deep_link::DeepLinkExt;
use tauri_plugin_opener::{open_path, OpenerExt};
use tokio::{fs, spawn};
use uuid::Uuid;
use {hostname, machine_uid};
use shared::app::shelf::module::{ResourceSelection, ShelfEvent};
use shared::app::transfer::module::TransferEvent;
use shared::entities::local_resource::LocalResourcePath;
use shared::entities::transfer_session::TransferType;
use crate::extensions::AppHandleExt;
use crate::mouse_tracking::{notify_user_did_drop, start_mouse_monitor, MouseMonitorConfig};
use crate::thumbnail::generate_thumbnail;

pub mod api;
pub mod extensions;
mod thumbnail;
mod mouse_tracking;

static CORE: LazyLock<Arc<Core<BitBridge>>> = LazyLock::new(|| Arc::new(Core::new()));
static TRAY_ICON: LazyLock<Mutex<Option<TrayIcon>>> = LazyLock::new(|| Mutex::new(None));
static TOAST_MESSAGE: LazyLock<Mutex<Option<String>>> = LazyLock::new(|| Mutex::new(None));

#[tauri::command]
async fn quit(app_handle: AppHandle) {
    app_handle.close_all_windows(vec![]);
}

#[tauri::command]
async fn cancel_send(app_handle: AppHandle, session_id: String) {
    let session_id = session_id.parse::<u64>().unwrap_or_default();
    process_event(TransferEvent::CancelTransfer {
        session_id,
        transfer_type: TransferType::send_any(),
    }, app_handle).await;
}

#[tauri::command]
async fn cancel_receive(app_handle: AppHandle, session_id: String) {
    let session_id = session_id.parse::<u64>().unwrap_or_default();
    process_event(TransferEvent::CancelTransfer {
        session_id,
        transfer_type: TransferType::Receive,
    }, app_handle).await;
}

#[tauri::command]
async fn clear_shelf(shelf_id: String, app_handle: AppHandle) {
    let shelf_id = shelf_id.parse::<u64>().unwrap_or_default();
    process_event(ShelfEvent::Clear { shelf_id }, app_handle).await;
}

#[tauri::command]
async fn public_transfer(shelf_id: String, password: Option<String>, app_handle: AppHandle) {
    let shelf_id = shelf_id.parse::<u64>().unwrap_or_default();
    process_event(TransferEvent::StartPublicTransfer {
        shelf_id,
        password,
        to_emails: vec![]
    }, app_handle).await;
}

#[tauri::command]
async fn email_transfer(shelf_id: String, password: Option<String>, to_emails: Vec<String>, app_handle: AppHandle) {
    let shelf_id = shelf_id.parse::<u64>().unwrap_or_default();
    process_event(TransferEvent::StartPublicTransfer {
        shelf_id,
        password,
        to_emails
    }, app_handle).await;
}

#[tauri::command]
async fn p2p_transfer(shelf_id: String, password: Option<String>, app_handle: AppHandle) {
    let shelf_id = shelf_id.parse::<u64>().unwrap_or_default();
    process_event(TransferEvent::StartP2PTransfer {
        shelf_id,
        nearby_available: false,
        password,
    }, app_handle).await;
}

#[tauri::command]
async fn ui_launched(app_handle: AppHandle) {
    render(CORE.view(), app_handle);
}

#[tauri::command]
async fn remove_resource(shelf_id: String, resource_id: String, app_handle: AppHandle) {
    let shelf_id = shelf_id.parse::<u64>().unwrap_or_default();
    let resource_id = resource_id.parse::<u64>().unwrap_or_default();
    process_event(ShelfEvent::RemoveResource { shelf_id, resource_id }, app_handle).await;
}

#[tauri::command]
async fn delete_receive_session(session_id: String, app_handle: AppHandle) {
    let session_id = session_id.parse::<u64>().unwrap_or_default();
    process_event(TransferEvent::DeleteSession {
        session_id,
    }, app_handle).await;
}

#[tauri::command]
async fn open_session(session_id: String, app_handle: AppHandle) {
    let session_id = session_id.parse::<u64>().unwrap_or_default();
    process_event(TransferEvent::OpenSession {
        session_id
    }, app_handle).await;
}

#[tauri::command]
async fn open_received_resource(session_id: String, resource_id: String, app_handle: AppHandle) {
    let session_id = session_id.parse::<u64>().unwrap_or_default();
    let resource_id = resource_id.parse::<u64>().unwrap_or_default();
    process_event(TransferEvent::OpenResource {
        session_id,
        resource_id
    }, app_handle).await;
}

#[tauri::command]
async fn open_shelf(app_handle: AppHandle) {
    notify_user_did_drop();
    app_handle.show_send();
}

#[tauri::command]
async fn get_or_create_shelf(shelf_id: String, app_handle: AppHandle) {
    log::info!("get_or_create_shelf called with shelf_id: {}", shelf_id);
    let shelf_id = shelf_id.parse::<u64>().unwrap_or_default();
    process_event(ShelfEvent::GetOrCreateShelf { shelf_id }, app_handle).await;
}

#[tauri::command]
async fn sign_out(app_handle: AppHandle) {
    process_event(AuthenticationEvent::SignOut, app_handle).await;
}

#[tauri::command]
async fn authenticate(app_handle: AppHandle) {
    process_event(AuthenticationEvent::Authenticate, app_handle).await;
}

#[tauri::command]
async fn add_resources(shelf_id: String, paths: Vec<String>, app_handle: AppHandle) {
    notify_user_did_drop();
    let shelf_id = shelf_id.parse::<u64>().unwrap_or_default();
    let selections = paths.into_iter().map(|path| ResourceSelection {
        path: LocalResourcePath::AbsolutePath(path),
        r#type: None
    }).collect::<Vec<_>>();

    process_event(ShelfEvent::AddResources { shelf_id, selections }, app_handle).await;
}

#[tauri::command]
fn get_toast_message() -> Option<String> {
    TOAST_MESSAGE.lock().ok().and_then(|guard| guard.clone())
}

#[tauri::command]
fn close_toast(app_handle: AppHandle) {
    if let Some(window) = app_handle.get_webview_window("toast") {
        let _ = window.close();
    }
    if let Ok(mut guard) = TOAST_MESSAGE.lock() {
        *guard = None;
    }
}

#[tauri::command]
async fn set_autostart(enabled: bool, app_handle: AppHandle) -> Result<(), String> {
    let autostart_manager = app_handle.autolaunch();
    if enabled {
        autostart_manager.enable().map_err(|e: tauri_plugin_autostart::Error| e.to_string())
    } else {
        autostart_manager.disable().map_err(|e: tauri_plugin_autostart::Error| e.to_string())
    }
}

#[tauri::command]
async fn is_autostart_enabled(app_handle: AppHandle) -> Result<bool, String> {
    let autostart_manager = app_handle.autolaunch();
    autostart_manager.is_enabled().map_err(|e: tauri_plugin_autostart::Error| e.to_string())
}

async fn process_event(event: impl Into<AppEvent> + Send + Sync + 'static, app_handle: AppHandle) {
    let effects = CORE.process_event(event.into());
    process_effects(effects, app_handle).await;
}

fn render(view: AppViewModel, app_handle: AppHandle) {
    let is_authorized = view.authentication.as_ref().map(|auth| auth.user.is_some()).unwrap_or(false);
    if !is_authorized {
        #[cfg(target_os = "macos")]
        let _ = app_handle.set_activation_policy(tauri::ActivationPolicy::Regular);
        app_handle.show_auth();
        update_tray_menu_signed_out(&app_handle);
    }
    else {
        #[cfg(target_os = "macos")]
        let _ = app_handle.set_activation_policy(tauri::ActivationPolicy::Accessory);
        app_handle.hide_auth();
        if let Some(shelf_view) = &view.shelf {
            update_tray_menu(&app_handle, &shelf_view.shelves);
        }
    }

    let _ = app_handle.emit("Render", view);
}

fn update_tray_menu_signed_out(app_handle: &AppHandle) {
    let Ok(quit_item) = MenuItemBuilder::with_id("quit", "Quit").build(app_handle) else { return };

    let Ok(menu) = MenuBuilder::new(app_handle)
        .item(&quit_item)
        .build() else { return };

    if let Ok(guard) = TRAY_ICON.lock() {
        if let Some(tray) = guard.as_ref() {
            let _ = tray.set_menu(Some(menu));
        }
    }
}

fn update_tray_menu(app_handle: &AppHandle, shelves: &[ShelfItemViewModel]) {
    let Ok(new_shelf_item) = MenuItemBuilder::with_id("new_shelf", "New Shelf").build(app_handle) else { return };
    let Ok(sign_out_item) = MenuItemBuilder::with_id("sign_out", "Sign out").build(app_handle) else { return };
    let Ok(quit_item) = MenuItemBuilder::with_id("quit", "Quit").build(app_handle) else { return };

    let mut recent_submenu_builder = SubmenuBuilder::with_id(app_handle, "recent_shelves", "Recent Shelves");
    for shelf in shelves.iter().take(10) {
        let shelf_id = format!("shelf_{}", shelf.id);
        let online_indicator = if shelf.is_online { "🟢 " } else { "" };
        let menu_text = format!("{}{} - {}", online_indicator, shelf.name, shelf.description);
        if let Ok(item) = MenuItemBuilder::with_id(&shelf_id, &menu_text).build(app_handle) {
            recent_submenu_builder = recent_submenu_builder.item(&item);
        }
    }

    let Ok(recent_submenu) = recent_submenu_builder.build() else { return };

    let Ok(menu) = MenuBuilder::new(app_handle)
        .item(&new_shelf_item)
        .item(&recent_submenu)
        .separator()
        .item(&sign_out_item)
        .item(&quit_item)
        .build() else { return };

    if let Ok(guard) = TRAY_ICON.lock() {
        if let Some(tray) = guard.as_ref() {
            let _ = tray.set_menu(Some(menu));
        }
    }
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
                        url: "bytover://".to_owned(),
                        unique_id: machine_uid::get().unwrap_or(Uuid::new_v4().to_string())
                    };

                    CORE.resolve(&mut handle, CoreOperationOutput::DeviceInfo(device)).unwrap_or_default()
                }
                DeviceOperation::GetGeoLocation => CORE.resolve(&mut handle, CoreOperationOutput::None).unwrap_or_default(),
                DeviceOperation::OpenSession(session) => {
                    let path = DiContainer::get_instance().path_resolver().get_session_dir_path(session).await;
                    open_path(path, Option::<&str>::None).unwrap_or_default();
                    CORE.resolve(&mut handle, CoreOperationOutput::None).unwrap_or_default()
                },
                DeviceOperation::Open(path) => {
                    if let LocalResourcePath::AbsolutePath(path) = path {
                        open_path(path, Option::<&str>::None).unwrap_or_default();
                    }

                    CORE.resolve(&mut handle, CoreOperationOutput::None).unwrap_or_default()
                },
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
                DeviceOperation::CloseShelf(shelf_id) => {
                    let label = format!("send-{}", shelf_id);
                    if let Some(window) = app_handle.get_webview_window(&label) {
                        let _ = window.close();
                    }
                    CORE.resolve(&mut handle, CoreOperationOutput::None).unwrap_or_default()
                }
            },
            CoreOperation::WebView(WebViewOperation::OpenUrl(url)) => {
                let _ = app_handle.opener().open_url(url, Option::<&str>::None);
                CORE.resolve(&mut handle, CoreOperationOutput::None).unwrap_or_default()
            }
            CoreOperation::Dialog(dialog) => match dialog {
                DialogOperation::Toast(msg) => {
                    log::info!(target: "toast", "{msg:?}");
                    if let Ok(mut guard) = TOAST_MESSAGE.lock() {
                        *guard = Some(msg.clone());
                    }

                    app_handle.show_toast(&msg);
                    CORE.resolve(&mut handle, CoreOperationOutput::None).unwrap_or_default()
                },
                DialogOperation::Alert(alert) => {
                    log::info!(target: "alert", "{alert:?}");
                    CORE.resolve(&mut handle, CoreOperationOutput::Bool(true)).unwrap_or_default()
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
        .plugin(tauri_plugin_positioner::init())
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, None))
        .invoke_handler(tauri::generate_handler![
            authenticate, add_resources,
            remove_resource, ui_launched, public_transfer, p2p_transfer, email_transfer,
            cancel_send, cancel_receive, delete_receive_session,
            open_received_resource, open_session, open_shelf,
            clear_shelf, sign_out, quit, get_or_create_shelf,
            get_toast_message, close_toast,
            set_autostart, is_autostart_enabled
        ])
        .setup(|app| {
            #[cfg(target_os = "macos")]
            let _ = app.handle().set_activation_policy(tauri::ActivationPolicy::Accessory);

            let quit_item = MenuItemBuilder::with_id("quit", "Quit").build(app)?;
            let menu = MenuBuilder::new(app)
                .item(&quit_item)
                .build()?;

            let tray_icon = include_bytes!("../icons/tray/icon.png");
            let icon = tauri::image::Image::from_bytes(tray_icon).expect("Failed to load tray icon");
            let tray = TrayIconBuilder::new()
                .icon(icon)
                .menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(|app, event| {
                    let event_id = event.id().as_ref();
                    match event_id {
                        "new_shelf" => {
                            notify_user_did_drop();
                            app.open_new_shelf_window();
                        },
                        "sign_out" => {
                            let app_handle = app.clone();
                            spawn(async move {
                                process_event(AuthenticationEvent::SignOut, app_handle).await;
                            });
                        },
                        "quit" => {
                            app.close_all_windows(vec![]);
                        },
                        id if id.starts_with("shelf_") => {
                            if let Some(shelf_id_str) = id.strip_prefix("shelf_") {
                                if let Ok(shelf_id) = shelf_id_str.parse::<u64>() {
                                    app.show_shelf(shelf_id);
                                }
                            }
                        },
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    tauri_plugin_positioner::on_tray_event(tray.app_handle(), &event);
                })
                .build(app)?;

            if let Ok(mut guard) = TRAY_ICON.lock() {
                *guard = Some(tray);
            }
            let handle = app.handle().clone();
            let workdir_path = app.path().app_data_dir().expect("We still solving issue that don't have app data dir");

            let access_url = var("BYTOVER_ACCESS_TOKEN").ok()
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

            let handle_cloned = handle.clone();
            spawn(async move {
                let handle = handle_cloned;
                let _ = fs::create_dir_all(&workdir_path);
                let bridge = Box::leak(Box::new(BridgeImpl {
                    app_handle: handle.clone()
                }));

                DiContainer::get_instance()
                    .init(Arc::new(PathResolverImpl::new(workdir_path).await), &*bridge)
                    .await;
                process_event(EnvironmentEvent::AppLaunched {
                    auto_launch_nearby: true,
                    allowed_nearby_anonymous: false
                }, handle.clone()).await;

                if let Some(url) = start_urls.first().cloned() {
                    let handle = handle.clone();
                    log::info!("Received redirect url: {}", url);
                    process_event(AuthenticationEvent::OnRedirected { url: url.to_string() }, handle).await;
                }
            });

            start_mouse_monitor(MouseMonitorConfig::default(), handle);
            #[cfg(target_os = "macos")]
            mouse_tracking::start_macos_drag_pasteboard_monitor();

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
