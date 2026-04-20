use core_services::utils::cancellation::{CancellationToken, FutureExtension, TaskErrors};
use file_icon_provider::get_file_icon;
use image::{DynamicImage, ImageFormat, RgbaImage};
use shared::entities::local_resource::ResourceType;
use std::path::PathBuf;
use std::time::Duration;
use tauri::async_runtime::spawn_blocking;
use thiserror::Error;
#[cfg(target_os = "macos")]
use tokio::process::Command;

#[derive(Error, Debug)]
pub enum ThumbnailError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Image processing error: {0}")]
    ImageError(#[from] image::ImageError),

    #[error("OS thumbnail API failed: {0}")]
    OsApiError(String),

    #[error("Invalid path")]
    InvalidPath,

    #[error("Cancelled")]
    Cancelled(#[from] TaskErrors),
}

/// Generate a thumbnail using OS-specific APIs with fallback to image crate
///
/// # Arguments
/// * `file_path` - Path to the source file
/// * `png_output_path` - Path where the PNG thumbnail will be saved
///
/// # Returns
/// * `Ok(())` if thumbnail generation succeeds
/// * `Err(ThumbnailError)` if generation fails
pub async fn generate_thumbnail(
    file_path: PathBuf,
    png_output_path: PathBuf,
    resource_type: &ResourceType,
) -> Result<(), ThumbnailError> {
    // Validate input paths
    if !file_path.exists() {
        return Err(ThumbnailError::InvalidPath);
    }

    if let Some(parent) = png_output_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    if matches!(resource_type, ResourceType::File | ResourceType::Folder) {
        // Load icon of files
        let icon = get_file_icon(file_path.clone(), 180).expect("Failed to get icon");
        if let Some(icon) = RgbaImage::from_raw(icon.width, icon.height, icon.pixels).map(DynamicImage::ImageRgba8) {
            let saved_path = spawn_blocking({
                let png_output_path = png_output_path.clone();
                move || {
                    icon.save_with_format(png_output_path.clone(), ImageFormat::Png).ok()?;
                    Some(png_output_path)
                }
            })
            .await
            .ok()
            .flatten();

            if saved_path.is_some() {
                return Ok(());
            }
        }
    }

    let cancellation = match resource_type {
        ResourceType::Video => CancellationToken::timeout(Duration::from_secs(6)),
        _ => CancellationToken::timeout(Duration::from_secs(2)),
    };

    // Try OS-specific thumbnail generation first
    match generate_os_thumbnail(&file_path, &png_output_path).with_cancel(&cancellation).await {
        Ok(Ok(_)) => return Ok(()),
        e => {
            log::warn!("OS thumbnail generation failed: {e:?}. Falling back to image crate.");
        }
    }

    // Fallback to image crate for cross-platform support
    generate_image_thumbnail(file_path, png_output_path).await
}

// ============================================================================
// macOS Implementation using QuickLook
// ============================================================================

#[cfg(target_os = "macos")]
async fn generate_os_thumbnail(file_path: &PathBuf, png_output_path: &PathBuf) -> Result<(), ThumbnailError> {
    // Get output directory for qlmanage
    let output_dir = png_output_path.parent().ok_or(ThumbnailError::InvalidPath)?;

    let output = Command::new("qlmanage")
        .arg("-t")           // Generate thumbnail
        .arg("-s")           // Size
        .arg("180")
        .arg("-o")           // Output directory
        .arg(output_dir)
        .arg(file_path)
        .kill_on_drop(true)
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ThumbnailError::OsApiError(format!("qlmanage failed: {}", stderr)));
    }

    // qlmanage creates a file with .png extension
    // The output filename is: <original_filename>.png
    let filename = file_path.file_name().ok_or(ThumbnailError::InvalidPath)?;
    let qlmanage_output = output_dir.join(format!("{}.png", filename.to_string_lossy()));

    if qlmanage_output.exists() {
        tokio::fs::rename(&qlmanage_output, png_output_path).await?;
        Ok(())
    } else {
        Err(ThumbnailError::OsApiError(
            "qlmanage did not create expected thumbnail file".to_string(),
        ))
    }
}

// ============================================================================
// Windows Implementation using IShellItemImageFactory
// ============================================================================

#[cfg(target_os = "windows")]
async fn generate_os_thumbnail(file_path: &PathBuf, png_output_path: &PathBuf) -> Result<(), ThumbnailError> {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::{Interface, PCWSTR};
    use windows::Win32::Foundation::SIZE;
    use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED};
    use windows::Win32::UI::Shell::{IShellItemImageFactory, SHCreateItemFromParsingName, SIIGBF_BIGGERSIZEOK};

    // Windows COM operations must be run in a blocking context
    let file_path = file_path.clone();
    let png_output_path = png_output_path.clone();

    tokio::task::spawn_blocking(move || {
        unsafe {
            // Initialize COM
            let hr = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            if hr.is_err() {
                return Err(ThumbnailError::OsApiError(format!("COM init failed: {:?}", hr)));
            }

            let result = (|| -> Result<(), ThumbnailError> {
                // Convert path to wide string
                let wide_path: Vec<u16> = file_path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();

                // Create IShellItem from path
                use windows::Win32::UI::Shell::IShellItem;

                let shell_item: IShellItem = SHCreateItemFromParsingName(PCWSTR(wide_path.as_ptr()), None)
                    .map_err(|e| ThumbnailError::OsApiError(format!("SHCreateItemFromParsingName failed: {}", e)))?;

                // Get IShellItemImageFactory interface
                let image_factory: IShellItemImageFactory = shell_item
                    .cast()
                    .map_err(|e| ThumbnailError::OsApiError(format!("Cast to IShellItemImageFactory failed: {}", e)))?;

                // Request thumbnail (512x512)
                let size = SIZE { cx: 512, cy: 512 };

                let hbitmap = image_factory
                    .GetImage(size, SIIGBF_BIGGERSIZEOK)
                    .map_err(|e| ThumbnailError::OsApiError(format!("GetImage failed: {}", e)))?;

                // Convert HBITMAP to PNG and save
                save_hbitmap_as_png(hbitmap, &png_output_path)?;

                Ok(())
            })();

            // Uninitialize COM
            CoUninitialize();

            result
        }
    })
    .await
    .map_err(|e| ThumbnailError::OsApiError(format!("Task join error: {}", e)))?
}

#[cfg(target_os = "windows")]
fn save_hbitmap_as_png(hbitmap: windows::Win32::Graphics::Gdi::HBITMAP, output_path: &PathBuf) -> Result<(), ThumbnailError> {
    use windows::Win32::Graphics::Gdi::{
        CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits, GetObjectW, BITMAP, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
        DIB_RGB_COLORS, HDC,
    };

    unsafe {
        // Get bitmap information
        let mut bitmap = BITMAP::default();
        let result = GetObjectW(
            hbitmap.into(),
            std::mem::size_of::<BITMAP>() as i32,
            Some(&mut bitmap as *mut _ as *mut _),
        );

        if result == 0 {
            DeleteObject(hbitmap.into());
            return Err(ThumbnailError::OsApiError("GetObjectW failed".to_string()));
        }

        let width = bitmap.bmWidth as u32;
        let height = bitmap.bmHeight as u32;

        // Create bitmap info for GetDIBits
        let mut bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width as i32,
                biHeight: -(height as i32), // negative for top-down DIB
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                biSizeImage: 0,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed: 0,
                biClrImportant: 0,
            },
            bmiColors: [Default::default(); 1],
        };

        // Allocate buffer for pixel data
        let buffer_size = (width * height * 4) as usize;
        let mut buffer = vec![0u8; buffer_size];

        // Get device context
        let hdc = CreateCompatibleDC(Some(HDC(std::ptr::null_mut())));
        if hdc.is_invalid() {
            DeleteObject(hbitmap.into());
            return Err(ThumbnailError::OsApiError("CreateCompatibleDC failed".to_string()));
        }

        // Get bitmap bits
        let lines = GetDIBits(
            hdc,
            hbitmap,
            0,
            height,
            Some(buffer.as_mut_ptr() as *mut _),
            &mut bmi,
            DIB_RGB_COLORS,
        );

        DeleteDC(hdc);

        if lines == 0 {
            DeleteObject(hbitmap.into());
            return Err(ThumbnailError::OsApiError("GetDIBits failed".to_string()));
        }

        // Convert BGRA to RGBA
        for i in (0..buffer.len()).step_by(4) {
            buffer.swap(i, i + 2); // Swap B and R
        }

        // Create image from raw buffer
        let img =
            image::RgbaImage::from_raw(width, height, buffer).ok_or(ThumbnailError::ImageError(image::ImageError::Parameter(
                image::error::ParameterError::from_kind(image::error::ParameterErrorKind::DimensionMismatch),
            )))?;

        // Ensure output directory exists
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Save as PNG
        img.save(output_path).map_err(|e| ThumbnailError::ImageError(e))?;

        DeleteObject(hbitmap.into());
        Ok(())
    }
}

// ============================================================================
// Linux Implementation using FreeDesktop Specification
// ============================================================================

#[cfg(target_os = "linux")]
async fn generate_os_thumbnail(file_path: &PathBuf, png_output_path: &PathBuf) -> Result<(), ThumbnailError> {
    use md5;

    // Follow FreeDesktop thumbnail specification
    // First check if thumbnail already exists in cache
    let uri = format!("file://{}", file_path.display());
    let digest = md5::compute(uri.as_bytes());
    let hash = format!("{:x}", digest);

    // Check standard FreeDesktop cache locations
    let cache_dir = dirs::cache_dir().ok_or(ThumbnailError::InvalidPath)?.join("thumbnails");

    // Try different sizes: large (180x180) first, then normal (180x180)
    for size_dir in &["large", "normal"] {
        let cached_thumb = cache_dir.join(size_dir).join(format!("{}.png", hash));

        if cached_thumb.exists() {
            // Verify timestamp matches
            let file_mtime = tokio::fs::metadata(file_path)
                .await?
                .modified()?
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|e| ThumbnailError::IoError(std::io::Error::new(std::io::ErrorKind::Other, e)))?
                .as_secs();

            // For simplicity, copy cached thumbnail if it exists
            // Production code should validate Thumb::MTime metadata
            tokio::fs::copy(&cached_thumb, png_output_path).await?;
            return Ok(());
        }
    }

    // No cached thumbnail found, try to use system thumbnailers
    // Check for common Linux thumbnailer tools
    let thumbnailer_check = Command::new("sh")
        .arg("-c")
        .arg("which tumbler 2>/dev/null || which gnome-thumbnail-factory 2>/dev/null || echo 'none'")
        .output()
        .await?;

    let thumbnailer = String::from_utf8_lossy(&thumbnailer_check.stdout).trim().to_string();

    if thumbnailer != "none" && !thumbnailer.is_empty() && thumbnailer != "" {
        // Create output directory
        if let Some(parent) = png_output_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Try using system thumbnailer (this is a simplified approach)
        // Real implementation would use D-Bus to communicate with thumbnailer daemon
        let result = Command::new("convert") // ImageMagick fallback
            .arg(file_path)
            .arg("-thumbnail")
            .arg("512x512>")
            .arg(png_output_path)
            .output()
            .await;

        if let Ok(output) = result {
            if output.status.success() && png_output_path.exists() {
                return Ok(());
            }
        }
    }

    // Fallback to image crate (will be called by parent function)
    Err(ThumbnailError::OsApiError("No system thumbnailer available".to_string()))
}

// ============================================================================
// Fallback Implementation using image crate
// ============================================================================

async fn generate_image_thumbnail(file_path: PathBuf, png_output_path: PathBuf) -> Result<(), ThumbnailError> {
    tokio::task::spawn_blocking(move || {
        // Open and decode image
        let img = image::open(&file_path)?;

        // Generate thumbnail (512x512 max dimensions, maintaining aspect ratio)
        let thumbnail = img.thumbnail(512, 512);

        // Ensure output directory exists
        if let Some(parent) = png_output_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Save as PNG
        thumbnail.save(&png_output_path)?;

        Ok::<(), ThumbnailError>(())
    })
    .await
    .map_err(|e| ThumbnailError::OsApiError(format!("Task join error: {}", e)))?
}
