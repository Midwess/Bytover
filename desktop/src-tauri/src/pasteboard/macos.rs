use cacao::foundation::{id, nil, NSData, NSString};
use cacao::objc::{msg_send, sel, sel_impl};
use cacao::pasteboard::{Pasteboard, PasteboardName, PasteboardType};
use dispatch::Queue;
use shared::app::shelf::module::ResourceSelection;
use shared::entities::local_resource::LocalResourcePath;
use tokio::fs;

use crate::content_handlers::{generate_filename, generate_redirect_html, get_dropped_content_path};

enum DragContent {
    Files(Vec<String>),
    ImageData(Vec<u8>),
    Url(String),
    Text(String),
    Empty,
}

pub async fn read_drag_pasteboard_selections() -> Result<Vec<ResourceSelection>, String> {
    let content = tokio::task::spawn_blocking(read_drag_content_on_main_thread)
        .await
        .map_err(|e| format!("Failed to read pasteboard: {}", e))?;

    match content {
        DragContent::Files(paths) => {
            log::info!("[pasteboard] Read {} file URLs from drag pasteboard", paths.len());
            Ok(paths
                .into_iter()
                .map(|path| ResourceSelection {
                    path: LocalResourcePath::AbsolutePath(path),
                    r#type: None,
                })
                .collect())
        }
        DragContent::ImageData(data) => {
            log::info!("[pasteboard] Read image data ({} bytes) from drag pasteboard", data.len());
            let filename = generate_filename("png");
            let file_path = get_dropped_content_path(&filename).await;

            fs::write(&file_path, &data).await.map_err(|e| format!("Failed to write image: {}", e))?;

            let path_str = file_path.to_string_lossy().to_string();
            Ok(vec![ResourceSelection {
                path: LocalResourcePath::AbsolutePath(path_str),
                r#type: None,
            }])
        }
        DragContent::Url(url) => {
            log::info!("[pasteboard] Read URL from drag pasteboard: {}", url);
            let html_content = generate_redirect_html(&url);
            let filename = generate_filename("html");
            let file_path = get_dropped_content_path(&filename).await;

            fs::write(&file_path, &html_content)
                .await
                .map_err(|e| format!("Failed to write redirect HTML: {}", e))?;

            let path_str = file_path.to_string_lossy().to_string();
            Ok(vec![ResourceSelection {
                path: LocalResourcePath::AbsolutePath(path_str),
                r#type: None,
            }])
        }
        DragContent::Text(text) => {
            log::info!("[pasteboard] Read text ({} chars) from drag pasteboard", text.len());
            let filename = generate_filename("txt");
            let file_path = get_dropped_content_path(&filename).await;

            fs::write(&file_path, &text).await.map_err(|e| format!("Failed to write text: {}", e))?;

            let path_str = file_path.to_string_lossy().to_string();
            Ok(vec![ResourceSelection {
                path: LocalResourcePath::AbsolutePath(path_str),
                r#type: None,
            }])
        }
        DragContent::Empty => {
            log::info!("[pasteboard] Drag pasteboard was empty");
            Ok(vec![])
        }
    }
}

fn read_drag_content_on_main_thread() -> DragContent {
    Queue::main().exec_sync(|| {
        let pb = Pasteboard::named(PasteboardName::Drag);

        if let Ok(urls) = pb.get_file_urls() {
            let paths: Vec<String> = urls
                .iter()
                .map(|url| url.absolute_string())
                .filter(|s| s.starts_with("file://"))
                .map(|s| percent_decode(&s.replacen("file://", "", 1)))
                .filter(|p| !p.is_empty())
                .collect();

            if !paths.is_empty() {
                return DragContent::Files(paths);
            }
        }

        if let Some(data) = read_data_for_type(&pb, PasteboardType::PNG) {
            if !data.is_empty() {
                return DragContent::ImageData(data);
            }
        }

        if let Some(data) = read_data_for_type(&pb, PasteboardType::TIFF) {
            if !data.is_empty() {
                return DragContent::ImageData(data);
            }
        }

        if let Some(url) = read_string_for_type(&pb, PasteboardType::URL) {
            let trimmed = url.trim();
            if !trimmed.is_empty() && (trimmed.starts_with("http://") || trimmed.starts_with("https://")) {
                return DragContent::Url(trimmed.to_string());
            }
        }

        if let Some(text) = read_string_for_type(&pb, PasteboardType::String) {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
                    return DragContent::Url(trimmed.to_string());
                }
                return DragContent::Text(trimmed.to_string());
            }
        }

        DragContent::Empty
    })
}

fn read_string_for_type(pb: &Pasteboard, ptype: PasteboardType) -> Option<String> {
    unsafe {
        let ptype_ns: NSString = ptype.into();
        let result: id = msg_send![&*pb.0, stringForType:&*ptype_ns];
        if result == nil {
            return None;
        }
        let ns_str = NSString::retain(result);
        Some(ns_str.to_string())
    }
}

fn read_data_for_type(pb: &Pasteboard, ptype: PasteboardType) -> Option<Vec<u8>> {
    unsafe {
        let ptype_ns: NSString = ptype.into();
        let result: id = msg_send![&*pb.0, dataForType:&*ptype_ns];
        if result == nil {
            return None;
        }
        let data = NSData::retain(result);
        if data.len() == 0 {
            return None;
        }
        Some(data.into_vec())
    }
}

fn percent_decode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(byte) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                result.push(byte as char);
                i += 3;
                continue;
            }
        }
        result.push(bytes[i] as char);
        i += 1;
    }
    result
}
