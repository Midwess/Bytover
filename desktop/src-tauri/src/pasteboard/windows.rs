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
    let content = tokio::task::spawn_blocking(read_drag_content)
        .await
        .map_err(|e| format!("Failed to read clipboard: {}", e))?;

    match content {
        DragContent::Files(paths) => {
            log::info!(
                "[pasteboard] Read {} file paths from drag clipboard",
                paths.len()
            );
            Ok(paths
                .into_iter()
                .map(|path| ResourceSelection {
                    path: LocalResourcePath::AbsolutePath(path),
                    r#type: None,
                })
                .collect())
        }
        DragContent::ImageData(data) => {
            log::info!(
                "[pasteboard] Read image data ({} bytes) from drag clipboard",
                data.len()
            );
            let filename = generate_filename("png");
            let file_path = get_dropped_content_path(&filename).await;

            fs::write(&file_path, &data)
                .await
                .map_err(|e| format!("Failed to write image: {}", e))?;

            let path_str = file_path.to_string_lossy().to_string();
            Ok(vec![ResourceSelection {
                path: LocalResourcePath::AbsolutePath(path_str),
                r#type: None,
            }])
        }
        DragContent::Url(url) => {
            log::info!("[pasteboard] Read URL from drag clipboard: {}", url);
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
            log::info!(
                "[pasteboard] Read text ({} chars) from drag clipboard",
                text.len()
            );
            let filename = generate_filename("txt");
            let file_path = get_dropped_content_path(&filename).await;

            fs::write(&file_path, &text)
                .await
                .map_err(|e| format!("Failed to write text: {}", e))?;

            let path_str = file_path.to_string_lossy().to_string();
            Ok(vec![ResourceSelection {
                path: LocalResourcePath::AbsolutePath(path_str),
                r#type: None,
            }])
        }
        DragContent::Empty => {
            log::info!("[pasteboard] Drag clipboard was empty");
            Ok(vec![])
        }
    }
}

fn read_drag_content() -> DragContent {
    use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED};

    log::info!("[pasteboard] read_drag_content: starting");

    unsafe {
        let hr = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        if hr.is_err() {
            log::warn!("[pasteboard] COM init failed: {:?}", hr);
            return DragContent::Empty;
        }
        log::info!("[pasteboard] COM initialized");

        let result = read_clipboard_data();

        CoUninitialize();
        log::info!("[pasteboard] COM uninitialized");
        result
    }
}

unsafe fn read_clipboard_data() -> DragContent {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::DataExchange::{CloseClipboard, OpenClipboard};

    if OpenClipboard(Some(HWND::default())).is_err() {
        log::warn!("[pasteboard] Failed to open clipboard (locked by another app)");
        return DragContent::Empty;
    }
    log::info!("[pasteboard] Clipboard opened");

    log::info!("[pasteboard] Trying CF_HDROP (files)...");
    if let Some(content) = try_read_files() {
        log::info!("[pasteboard] CF_HDROP returned: {:?}", match &content {
            DragContent::Files(p) => format!("Files({})", p.len()),
            _ => "?".to_string(),
        });
        let _ = CloseClipboard();
        return content;
    }
    log::info!("[pasteboard] CF_HDROP: no files found");

    log::info!("[pasteboard] Trying CF_DIB (image)...");
    if let Some(content) = try_read_image() {
        log::info!("[pasteboard] CF_DIB returned: {:?}", match &content {
            DragContent::ImageData(d) => format!("ImageData({} bytes)", d.len()),
            _ => "?".to_string(),
        });
        let _ = CloseClipboard();
        return content;
    }
    log::info!("[pasteboard] CF_DIB: no image data found");

    log::info!("[pasteboard] Trying CF_UNICODETEXT (text)...");
    let result = try_read_text()
        .unwrap_or_else(|| {
            log::info!("[pasteboard] CF_UNICODETEXT: no text found");
            DragContent::Empty
        });
    if !matches!(result, DragContent::Empty) {
        log::info!("[pasteboard] CF_UNICODETEXT returned: {:?}", match &result {
            DragContent::Url(u) => format!("Url({})", u),
            DragContent::Text(t) => format!("Text({} chars)", t.len()),
            _ => "?".to_string(),
        });
    }

    let _ = CloseClipboard();
    result
}

const CF_HDROP: u32 = 15;
const CF_DIB: u32 = 8;
const CF_UNICODETEXT: u32 = 13;

unsafe fn try_read_files() -> Option<DragContent> {
    use windows::Win32::System::DataExchange::GetClipboardData;
    use windows::Win32::UI::Shell::{DragQueryFileW, HDROP};

    let handle = match GetClipboardData(CF_HDROP) {
        Ok(h) => {
            log::info!("[pasteboard] CF_HDROP handle: {:?}", h);
            h
        }
        Err(e) => {
            log::info!("[pasteboard] CF_HDROP: GetClipboardData failed: {:?}", e);
            return None;
        }
    };
    let hdrop = HDROP(handle.0 as _);

    let count = DragQueryFileW(hdrop, 0xFFFFFFFF, None);
    log::info!("[pasteboard] CF_HDROP: {} file(s) in drop", count);
    if count == 0 {
        return None;
    }

    let mut paths = Vec::new();
    for i in 0..count {
        let len = DragQueryFileW(hdrop, i, None);
        if len == 0 {
            continue;
        }
        let mut buf = vec![0u16; (len + 1) as usize];
        DragQueryFileW(hdrop, i, Some(&mut buf));
        let path = String::from_utf16_lossy(&buf[..len as usize]);
        if !path.is_empty() {
            log::info!("[pasteboard] CF_HDROP:   file[{}] = {}", paths.len(), path);
            paths.push(path);
        }
    }

    if paths.is_empty() {
        log::info!("[pasteboard] CF_HDROP: all paths were empty");
        None
    } else {
        log::info!("[pasteboard] CF_HDROP: returning {} valid paths", paths.len());
        Some(DragContent::Files(paths))
    }
}

unsafe fn try_read_image() -> Option<DragContent> {
    use windows::Win32::Foundation::HGLOBAL;
    use windows::Win32::System::DataExchange::GetClipboardData;
    use windows::Win32::System::Memory::{GlobalLock, GlobalSize, GlobalUnlock};

    let handle = match GetClipboardData(CF_DIB) {
        Ok(h) => {
            log::info!("[pasteboard] CF_DIB handle: {:?}", h);
            h
        }
        Err(e) => {
            log::info!("[pasteboard] CF_DIB: GetClipboardData failed: {:?}", e);
            return None;
        }
    };
    let hglobal = HGLOBAL(handle.0 as _);

    let size = GlobalSize(hglobal);
    log::info!("[pasteboard] CF_DIB: GlobalSize = {} bytes", size);
    if size == 0 {
        log::info!("[pasteboard] CF_DIB: size is 0");
        return None;
    }

    let ptr = GlobalLock(hglobal);
    if ptr.is_null() {
        log::info!("[pasteboard] CF_DIB: GlobalLock returned null");
        return None;
    }

    let dib_data = std::slice::from_raw_parts(ptr as *const u8, size);
    log::info!("[pasteboard] CF_DIB: locked {} bytes, converting to PNG...", size);
    let png_bytes = dib_to_png(dib_data);

    let _ = GlobalUnlock(hglobal);

    match png_bytes {
        Some(bytes) => {
            log::info!("[pasteboard] CF_DIB: PNG conversion successful, {} bytes", bytes.len());
            Some(DragContent::ImageData(bytes))
        }
        None => {
            log::info!("[pasteboard] CF_DIB: PNG conversion failed");
            None
        }
    }
}

fn dib_to_png(dib_data: &[u8]) -> Option<Vec<u8>> {
    use std::io::Cursor;

    if dib_data.len() < 40 {
        return None;
    }

    let bi_size =
        u32::from_le_bytes([dib_data[0], dib_data[1], dib_data[2], dib_data[3]]) as usize;
    let bi_bit_count = u16::from_le_bytes([dib_data[14], dib_data[15]]) as usize;
    let bi_clr_used =
        u32::from_le_bytes([dib_data[32], dib_data[33], dib_data[34], dib_data[35]]) as usize;

    let color_table_entries = if bi_clr_used > 0 {
        bi_clr_used
    } else if bi_bit_count <= 8 {
        1usize << bi_bit_count
    } else {
        0
    };

    let pixel_offset = 14 + bi_size + (color_table_entries * 4);
    let file_size = 14 + dib_data.len();

    let mut bmp = Vec::with_capacity(file_size);
    bmp.extend_from_slice(&[0x42, 0x4D]);
    bmp.extend_from_slice(&(file_size as u32).to_le_bytes());
    bmp.extend_from_slice(&[0u8; 4]);
    bmp.extend_from_slice(&(pixel_offset as u32).to_le_bytes());
    bmp.extend_from_slice(dib_data);

    let img = image::load_from_memory_with_format(&bmp, image::ImageFormat::Bmp).ok()?;
    let mut png_buf = Cursor::new(Vec::new());
    img.write_to(&mut png_buf, image::ImageFormat::Png).ok()?;

    let bytes = png_buf.into_inner();
    if bytes.is_empty() {
        None
    } else {
        Some(bytes)
    }
}

unsafe fn try_read_text() -> Option<DragContent> {
    use windows::Win32::Foundation::HGLOBAL;
    use windows::Win32::System::DataExchange::GetClipboardData;
    use windows::Win32::System::Memory::{GlobalLock, GlobalUnlock};

    let handle = match GetClipboardData(CF_UNICODETEXT) {
        Ok(h) => {
            log::info!("[pasteboard] CF_UNICODETEXT handle: {:?}", h);
            h
        }
        Err(e) => {
            log::info!("[pasteboard] CF_UNICODETEXT: GetClipboardData failed: {:?}", e);
            return None;
        }
    };
    let hglobal = HGLOBAL(handle.0 as _);

    let ptr = GlobalLock(hglobal);
    if ptr.is_null() {
        log::info!("[pasteboard] CF_UNICODETEXT: GlobalLock returned null");
        return None;
    }

    let wide_ptr = ptr as *const u16;
    let mut len = 0;
    while *wide_ptr.add(len) != 0 {
        len += 1;
        if len > 1_000_000 {
            break;
        }
    }
    log::info!("[pasteboard] CF_UNICODETEXT: locked, {} chars (max 1M)", len);

    let slice = std::slice::from_raw_parts(wide_ptr, len);
    let text = String::from_utf16_lossy(slice);

    let _ = GlobalUnlock(hglobal);

    let trimmed = text.trim();
    log::info!("[pasteboard] CF_UNICODETEXT: text = \"{}\" ({} raw, {} trimmed)", &text[..text.len().min(200)], text.len(), trimmed.len());
    if trimmed.is_empty() {
        log::info!("[pasteboard] CF_UNICODETEXT: text is empty after trim");
        return None;
    }

    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        log::info!("[pasteboard] CF_UNICODETEXT: detected URL, returning Url variant");
        Some(DragContent::Url(trimmed.to_string()))
    } else {
        log::info!("[pasteboard] CF_UNICODETEXT: plain text, returning Text variant");
        Some(DragContent::Text(trimmed.to_string()))
    }
}
