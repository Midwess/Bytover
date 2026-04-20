use native::di_container::DiContainer;
use shared::app::shelf::module::{ResourceSelection, ShelfEvent};
use shared::entities::local_resource::LocalResourcePath;
use std::path::PathBuf;
use std::time::Duration;
use tauri::{AppHandle, Manager};
use tauri_plugin_clipboard::Clipboard;
use tokio::fs::{self, File};
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

const HTTP_TIMEOUT_SECS: u64 = 30;

use crate::mouse_tracking::notify_user_did_drop;
use crate::process_event;

pub(crate) fn generate_filename(extension: &str) -> String {
    let timestamp = chrono::Utc::now().timestamp_millis();
    let short_uuid = &Uuid::new_v4().to_string()[..8];
    format!("generated_{}_{}.{}", timestamp, short_uuid, extension)
}

pub(crate) fn generate_redirect_html(url: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <meta http-equiv="refresh" content="0; url={}">
</head>
</html>"#,
        url
    )
}

pub(crate) async fn get_dropped_content_path(filename: &str) -> PathBuf {
    let dir = DiContainer::get_instance().path_resolver().get_dropped_content_dir_path().await;
    PathBuf::from(dir).join(filename)
}

async fn fetch_url_with_timeout(url: &str) -> Result<reqwest::Response, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(HTTP_TIMEOUT_SECS))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    client.get(url).send().await.map_err(|e| format!("Failed to download: {}", e))
}

async fn stream_response_to_file(mut response: reqwest::Response, file_path: &PathBuf) -> Result<(), String> {
    let mut file = File::create(file_path).await.map_err(|e| format!("Failed to create file: {}", e))?;

    while let Some(chunk) = response.chunk().await.map_err(|e| format!("Failed to read chunk: {}", e))? {
        file.write_all(&chunk).await.map_err(|e| format!("Failed to write chunk: {}", e))?;
    }

    file.flush().await.map_err(|e| format!("Failed to flush file: {}", e))?;

    Ok(())
}

async fn add_resource_from_path(shelf_id: u64, path: String, app_handle: AppHandle) {
    let selections = vec![ResourceSelection {
        path: LocalResourcePath::AbsolutePath(path),
        r#type: None,
    }];
    process_event(ShelfEvent::AddResources { shelf_id, selections }, app_handle).await;
}

#[tauri::command]
pub async fn add_url_resource(shelf_id: String, url: String, app_handle: AppHandle) -> Result<(), String> {
    log::info!(
        "[content_handlers] add_url_resource called - shelf_id: {}, url: {}",
        shelf_id,
        url
    );
    notify_user_did_drop();
    let shelf_id = shelf_id.parse::<u64>().map_err(|e| e.to_string())?;

    let response = fetch_url_with_timeout(&url).await?;

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
        _ => url
            .split('/')
            .last()
            .and_then(|s| s.split('.').last())
            .filter(|ext| ext.len() <= 5 && ext.chars().all(|c| c.is_alphanumeric()))
            .unwrap_or("bin"),
    };

    let filename = generate_filename(extension);
    let file_path = get_dropped_content_path(&filename).await;

    stream_response_to_file(response, &file_path).await?;

    let path_str = file_path.to_string_lossy().to_string();
    add_resource_from_path(shelf_id, path_str, app_handle).await;

    Ok(())
}

#[tauri::command]
pub async fn add_text_resource(shelf_id: String, content: String, app_handle: AppHandle) -> Result<(), String> {
    log::info!(
        "[content_handlers] add_text_resource called - shelf_id: {}, content_len: {}",
        shelf_id,
        content.len()
    );
    notify_user_did_drop();
    let shelf_id = shelf_id.parse::<u64>().map_err(|e| e.to_string())?;

    let filename = generate_filename("txt");
    let file_path = get_dropped_content_path(&filename).await;

    fs::write(&file_path, &content).await.map_err(|e| format!("Failed to write file: {}", e))?;

    let path_str = file_path.to_string_lossy().to_string();
    add_resource_from_path(shelf_id, path_str, app_handle).await;

    Ok(())
}

#[tauri::command]
pub async fn add_html_resource(shelf_id: String, content: String, app_handle: AppHandle) -> Result<(), String> {
    log::info!(
        "[content_handlers] add_html_resource called - shelf_id: {}, content_len: {}",
        shelf_id,
        content.len()
    );
    notify_user_did_drop();
    let shelf_id = shelf_id.parse::<u64>().map_err(|e| e.to_string())?;

    let filename = generate_filename("html");
    let file_path = get_dropped_content_path(&filename).await;

    fs::write(&file_path, &content).await.map_err(|e| format!("Failed to write file: {}", e))?;

    let path_str = file_path.to_string_lossy().to_string();
    add_resource_from_path(shelf_id, path_str, app_handle).await;

    Ok(())
}

pub async fn read_clipboard_selections(app_handle: &AppHandle) -> Result<Vec<ResourceSelection>, String> {
    log::info!("[content_handlers] read_clipboard_selections called");

    let clipboard = app_handle.state::<Clipboard>();

    if let Ok(files) = clipboard.read_files() {
        log::info!("[content_handlers] clipboard.read_files() returned {} files", files.len());
        if !files.is_empty() {
            let selections = files
                .into_iter()
                .map(|f| ResourceSelection {
                    path: LocalResourcePath::AbsolutePath(f.replace("file://", "")),
                    r#type: None,
                })
                .collect::<Vec<_>>();

            return Ok(selections);
        }
    }

    if let Ok(img) = clipboard.read_image_binary() {
        let filename = generate_filename("png");
        let file_path = get_dropped_content_path(&filename).await;

        fs::write(&file_path, &img).await.map_err(|e| format!("Failed to write image: {}", e))?;

        let path_str = file_path.to_string_lossy().to_string();
        return Ok(vec![ResourceSelection {
            path: LocalResourcePath::AbsolutePath(path_str),
            r#type: None,
        }]);
    }

    if let Ok(text) = clipboard.read_text() {
        if text.trim().is_empty() {
            return Ok(vec![]);
        }

        if text.starts_with("http://") || text.starts_with("https://") {
            let html_content = generate_redirect_html(&text);
            let filename = generate_filename("html");
            let file_path = get_dropped_content_path(&filename).await;

            fs::write(&file_path, &html_content)
                .await
                .map_err(|e| format!("Failed to write redirect HTML: {}", e))?;

            let path_str = file_path.to_string_lossy().to_string();
            return Ok(vec![ResourceSelection {
                path: LocalResourcePath::AbsolutePath(path_str),
                r#type: None,
            }]);
        }

        let filename = generate_filename("txt");
        let file_path = get_dropped_content_path(&filename).await;

        fs::write(&file_path, &text).await.map_err(|e| format!("Failed to write text: {}", e))?;

        let path_str = file_path.to_string_lossy().to_string();
        return Ok(vec![ResourceSelection {
            path: LocalResourcePath::AbsolutePath(path_str),
            r#type: None,
        }]);
    }

    Ok(vec![])
}

#[tauri::command]
pub async fn paste_from_clipboard(shelf_id: String, app_handle: AppHandle) -> Result<(), String> {
    notify_user_did_drop();
    let shelf_id_u64 = shelf_id.parse::<u64>().map_err(|e| e.to_string())?;

    let selections = read_clipboard_selections(&app_handle).await?;
    if !selections.is_empty() {
        process_event(
            ShelfEvent::AddResources {
                shelf_id: shelf_id_u64,
                selections,
            },
            app_handle,
        )
        .await;
    }
    Ok(())
}
