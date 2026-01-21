use native::di_container::DiContainer;
use shared::app::shelf::module::{ResourceSelection, ShelfEvent};
use shared::entities::local_resource::LocalResourcePath;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};
use tauri_plugin_clipboard::Clipboard;
use tokio::fs;
use uuid::Uuid;

use crate::process_event;
use crate::mouse_tracking::notify_user_did_drop;

fn generate_filename(extension: &str) -> String {
    let timestamp = chrono::Utc::now().timestamp_millis();
    let short_uuid = &Uuid::new_v4().to_string()[..8];
    format!("dropped_{}_{}.{}", timestamp, short_uuid, extension)
}

async fn get_dropped_content_path(filename: &str) -> PathBuf {
    let dir = DiContainer::get_instance()
        .path_resolver()
        .get_dropped_content_dir_path()
        .await;
    PathBuf::from(dir).join(filename)
}

async fn add_resource_from_path(shelf_id: u64, path: String, app_handle: AppHandle) {
    let selections = vec![ResourceSelection {
        path: LocalResourcePath::AbsolutePath(path),
        r#type: None,
    }];
    process_event(
        ShelfEvent::AddResources {
            shelf_id,
            selections,
        },
        app_handle,
    )
    .await;
}

#[tauri::command]
pub async fn add_url_resource(
    shelf_id: String,
    url: String,
    app_handle: AppHandle,
) -> Result<(), String> {
    log::info!("[content_handlers] add_url_resource called - shelf_id: {}, url: {}", shelf_id, url);
    notify_user_did_drop();
    let shelf_id = shelf_id.parse::<u64>().map_err(|e| e.to_string())?;

    let response = reqwest::get(&url)
        .await
        .map_err(|e| format!("Failed to download: {}", e))?;

    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/octet-stream");

    let extension = match content_type {
        t if t.starts_with("image/png") => "png",
        t if t.starts_with("image/jpeg") => "jpg",
        t if t.starts_with("image/gif") => "gif",
        t if t.starts_with("image/webp") => "webp",
        t if t.starts_with("image/svg") => "svg",
        t if t.starts_with("text/html") => "html",
        t if t.starts_with("text/plain") => "txt",
        t if t.starts_with("application/pdf") => "pdf",
        _ => {
            url.split('/')
                .last()
                .and_then(|s| s.split('.').last())
                .filter(|ext| ext.len() <= 5 && ext.chars().all(|c| c.is_alphanumeric()))
                .unwrap_or("bin")
        }
    };

    let filename = generate_filename(extension);
    let file_path = get_dropped_content_path(&filename).await;

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;

    fs::write(&file_path, &bytes)
        .await
        .map_err(|e| format!("Failed to write file: {}", e))?;

    let path_str = file_path.to_string_lossy().to_string();
    add_resource_from_path(shelf_id, path_str, app_handle).await;

    Ok(())
}

#[tauri::command]
pub async fn add_text_resource(
    shelf_id: String,
    content: String,
    app_handle: AppHandle,
) -> Result<(), String> {
    log::info!("[content_handlers] add_text_resource called - shelf_id: {}, content_len: {}", shelf_id, content.len());
    notify_user_did_drop();
    let shelf_id = shelf_id.parse::<u64>().map_err(|e| e.to_string())?;

    let filename = generate_filename("txt");
    let file_path = get_dropped_content_path(&filename).await;

    fs::write(&file_path, &content)
        .await
        .map_err(|e| format!("Failed to write file: {}", e))?;

    let path_str = file_path.to_string_lossy().to_string();
    add_resource_from_path(shelf_id, path_str, app_handle).await;

    Ok(())
}

#[tauri::command]
pub async fn add_html_resource(
    shelf_id: String,
    content: String,
    app_handle: AppHandle,
) -> Result<(), String> {
    log::info!("[content_handlers] add_html_resource called - shelf_id: {}, content_len: {}", shelf_id, content.len());
    notify_user_did_drop();
    let shelf_id = shelf_id.parse::<u64>().map_err(|e| e.to_string())?;

    let filename = generate_filename("html");
    let file_path = get_dropped_content_path(&filename).await;

    fs::write(&file_path, &content)
        .await
        .map_err(|e| format!("Failed to write file: {}", e))?;

    let path_str = file_path.to_string_lossy().to_string();
    add_resource_from_path(shelf_id, path_str, app_handle).await;

    Ok(())
}

#[tauri::command]
pub async fn paste_from_clipboard(
    shelf_id: String,
    app_handle: AppHandle,
) -> Result<(), String> {
    log::info!("[content_handlers] paste_from_clipboard called - shelf_id: {}", shelf_id);
    notify_user_did_drop();
    let shelf_id_u64 = shelf_id.parse::<u64>().map_err(|e| e.to_string())?;

    let clipboard = app_handle.state::<Clipboard>();

    if let Ok(files) = clipboard.read_files() {
        log::info!("[content_handlers] clipboard.read_files() returned {} files", files.len());
        if !files.is_empty() {
            let paths: Vec<String> = files
                .into_iter()
                .map(|f| f.replace("file://", ""))
                .collect();

            let selections = paths
                .into_iter()
                .map(|path| ResourceSelection {
                    path: LocalResourcePath::AbsolutePath(path),
                    r#type: None,
                })
                .collect::<Vec<_>>();

            process_event(
                ShelfEvent::AddResources {
                    shelf_id: shelf_id_u64,
                    selections,
                },
                app_handle,
            )
            .await;
            return Ok(());
        }
    }

    if let Ok(img) = clipboard.read_image_binary() {
        let filename = generate_filename("png");
        let file_path = get_dropped_content_path(&filename).await;

        fs::write(&file_path, &img)
            .await
            .map_err(|e| format!("Failed to write image: {}", e))?;

        let path_str = file_path.to_string_lossy().to_string();
        add_resource_from_path(shelf_id_u64, path_str, app_handle).await;
        return Ok(());
    }

    if let Ok(text) = clipboard.read_text() {
        if text.trim().is_empty() {
            return Ok(());
        }

        if text.starts_with("http://") || text.starts_with("https://") {
            return add_url_resource(shelf_id, text, app_handle).await;
        }

        let filename = generate_filename("txt");
        let file_path = get_dropped_content_path(&filename).await;

        fs::write(&file_path, &text)
            .await
            .map_err(|e| format!("Failed to write text: {}", e))?;

        let path_str = file_path.to_string_lossy().to_string();
        add_resource_from_path(shelf_id_u64, path_str, app_handle).await;
        return Ok(());
    }

    Ok(())
}
