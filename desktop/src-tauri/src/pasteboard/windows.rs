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

unsafe fn enumerate_clipboard_formats() {
    use windows::Win32::System::DataExchange::EnumClipboardFormats;

    let mut fmt: u32 = 0;
    let mut found = Vec::new();
    loop {
        fmt = EnumClipboardFormats(fmt);
        if fmt == 0 {
            break;
        }
        found.push(fmt);
    }
    log::info!("[pasteboard] Clipboard formats available: {:?}", found);
}

unsafe fn read_clipboard_data() -> DragContent {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::DataExchange::{CloseClipboard, OpenClipboard};

    if OpenClipboard(Some(HWND::default())).is_err() {
        log::warn!("[pasteboard] Failed to open clipboard (locked by another app)");
        return DragContent::Empty;
    }
    log::info!("[pasteboard] Clipboard opened");

    enumerate_clipboard_formats();

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
const CF_BITMAP: u32 = 2;
const CF_ENHMETAFILE: u32 = 14;

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
    if let Some(img) = try_read_cf_dib() {
        return Some(img);
    }
    if let Some(img) = try_read_cf_bitmap() {
        return Some(img);
    }
    if let Some(img) = try_read_cf_enhmetafile() {
        return Some(img);
    }
    None
}

unsafe fn try_read_cf_dib() -> Option<DragContent> {
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
    if size < 40 {
        log::info!("[pasteboard] CF_DIB: size {} < 40 (header only), skipping", size);
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

unsafe fn try_read_cf_bitmap() -> Option<DragContent> {
    use windows::Win32::Foundation::{HBITMAP, HDC};
    use windows::Win32::Graphics::Gdi::{CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits, GetObjectW, SelectObject, BITMAP, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS};
    use windows::Win32::System::DataExchange::GetClipboardData;
    use windows::Win32::System::Memory::{GlobalAlloc, GlobalFree, GMEM_MOVEABLE};
    use std::io::Cursor;

    log::info!("[pasteboard] CF_BITMAP: trying...");
    let handle = match GetClipboardData(CF_BITMAP) {
        Ok(h) => {
            log::info!("[pasteboard] CF_BITMAP handle: {:?}", h);
            h
        }
        Err(e) => {
            log::info!("[pasteboard] CF_BITMAP: GetClipboardData failed: {:?}", e);
            return None;
        }
    };

    let hbitmap = HBITMAP(handle.0 as _);
    let mut bm = std::mem::zeroed::<BITMAP>();
    let bytes_copied = GetObjectW(hbitmap, std::mem::size_of::<BITMAP>() as i32, Some(&mut bm as *mut _ as *mut _));
    if bytes_copied == 0 {
        log::info!("[pasteboard] CF_BITMAP: GetObjectW failed");
        return None;
    }

    let width = bm.bmWidth.abs() as u32;
    let height = bm.bmHeight.abs() as u32;
    let bpp = bm.bmBitsPixel as u32;
    let row_size = ((width * bpp + 31) / 32) * 4;
    log::info!("[pasteboard] CF_BITMAP: {}x{}, bpp={}, row_size={}", width, height, bpp, row_size);

    let screen_dc = HDC(std::ptr::null_mut());
    let mem_dc = CreateCompatibleDC(screen_dc);
    if mem_dc.is_invalid() {
        log::info!("[pasteboard] CF_BITMAP: CreateCompatibleDC failed");
        return None;
    }
    let old_bitmap = SelectObject(mem_dc, hbitmap);

    let mut bmi = std::mem::zeroed::<BITMAPINFO>();
    bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
    bmi.bmiHeader.biWidth = width as i32;
    bmi.bmiHeader.biHeight = -(height as i32);
    bmi.bmiHeader.biPlanes = 1;
    bmi.bmiHeader.biBitCount = bpp as u16;
    bmi.bmiHeader.biCompression = BI_RGB.0 as u32;

    let pixels_size = (row_size * height) as usize;
    let hglobal = GlobalAlloc(GMEM_MOVEABLE, pixels_size);
    if hglobal.is_null() {
        log::info!("[pasteboard] CF_BITMAP: GlobalAlloc failed");
        let _ = SelectObject(mem_dc, old_bitmap);
        let _ = DeleteDC(mem_dc);
        return None;
    }

    let pixels = std::slice::from_raw_parts_mut(GlobalLock(hglobal) as *mut u8, pixels_size);
    let lines = GetDIBits(
        mem_dc,
        hbitmap,
        0,
        height,
        Some(pixels),
        &mut bmi,
        DIB_RGB_COLORS,
    );
    let _ = GlobalUnlock(hglobal);

    let _ = SelectObject(mem_dc, old_bitmap);
    let _ = DeleteDC(mem_dc);
    let _ = DeleteObject(HBITMAP(handle.0 as _));
    let _ = GlobalFree(hglobal);

    if lines == 0 {
        log::info!("[pasteboard] CF_BITMAP: GetDIBits returned 0 lines");
        return None;
    }
    log::info!("[pasteboard] CF_BITMAP: GetDIBits got {} lines", lines);

    let img = match bpp {
        32 => {
            let mut rgba = vec![0u8; (width * height * 4) as usize];
            for y in 0..height {
                for x in 0..width {
                    let src = (y * row_size + x * 4) as usize;
                    let dst = ((height - 1 - y) * width + x) as usize * 4;
                    if src + 3 < pixels.len() && dst + 3 < rgba.len() {
                        rgba[dst] = pixels[src + 2];
                        rgba[dst + 1] = pixels[src + 1];
                        rgba[dst + 2] = pixels[src];
                        rgba[dst + 3] = pixels[src + 3];
                    }
                }
            }
            image::DynamicImage::ImageRgba8(image::RgbaImage::from_raw(width, height, rgba)?)
        }
        24 => {
            let mut rgb = vec![0u8; (width * height * 3) as usize];
            for y in 0..height {
                for x in 0..width {
                    let src = (y * row_size + x * 3) as usize;
                    let dst = ((height - 1 - y) * width + x) as usize * 3;
                    if src + 2 < pixels.len() && dst + 2 < rgb.len() {
                        rgb[dst] = pixels[src + 2];
                        rgb[dst + 1] = pixels[src + 1];
                        rgb[dst + 2] = pixels[src];
                    }
                }
            }
            image::DynamicImage::ImageRgb8(image::RgbImage::from_raw(width, height, rgb)?)
        }
        _ => {
            log::info!("[pasteboard] CF_BITMAP: unsupported bpp={}", bpp);
            return None;
        }
    };

    let mut png_buf = Cursor::new(Vec::new());
    img.write_to(&mut png_buf, image::ImageFormat::Png).ok()?;
    let bytes = png_buf.into_inner();
    if bytes.is_empty() {
        log::info!("[pasteboard] CF_BITMAP: PNG encoding empty");
        None
    } else {
        log::info!("[pasteboard] CF_BITMAP: PNG conversion successful, {} bytes", bytes.len());
        Some(DragContent::ImageData(bytes))
    }
}

unsafe fn try_read_cf_enhmetafile() -> Option<DragContent> {
    use windows::Win32::Graphics::Gdi::{DeleteEnhMetaFile, GetEnhMetaFileBits, HENHMETAFILE};
    use windows::Win32::System::DataExchange::GetClipboardData;
    use std::io::Cursor;

    log::info!("[pasteboard] CF_ENHMETAFILE: trying...");
    let handle = match GetClipboardData(CF_ENHMETAFILE) {
        Ok(h) => {
            log::info!("[pasteboard] CF_ENHMETAFILE handle: {:?}", h);
            h
        }
        Err(e) => {
            log::info!("[pasteboard] CF_ENHMETAFILE: GetClipboardData failed: {:?}", e);
            return None;
        }
    };
    let hemf = HENHMETAFILE(handle.0 as _);

    let size = GetEnhMetaFileBits(hemf, None);
    log::info!("[pasteboard] CF_ENHMETAFILE: size = {} bytes", size);
    if size == 0 {
        return None;
    }

    let mut buf = vec![0u8; size as usize];
    let actual = GetEnhMetaFileBits(hemf, Some(&mut buf));
    let _ = DeleteEnhMetaFile(hemf);

    if actual != size {
        log::info!("[pasteboard] CF_ENHMETAFILE: GetEnhMetaFileBits returned {} != {}", actual, size);
        return None;
    }

    let mut png_buf = Cursor::new(Vec::new());
    if let Some(emf_img) = emf_to_image(&buf) {
        emf_img.write_to(&mut png_buf, image::ImageFormat::Png).ok()?;
        let bytes = png_buf.into_inner();
        if !bytes.is_empty() {
            log::info!("[pasteboard] CF_ENHMETAFILE: PNG conversion successful, {} bytes", bytes.len());
            return Some(DragContent::ImageData(bytes));
        }
    }
    log::info!("[pasteboard] CF_ENHMETAFILE: EMF->PNG conversion failed");
    None
}

fn emf_to_image(_buf: &[u8]) -> Option<image::DynamicImage> {
    None
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
