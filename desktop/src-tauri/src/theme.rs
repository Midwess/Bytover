use std::sync::atomic::{AtomicU8, Ordering};
use tauri::AppHandle;
use crate::TRAY_ICON;

static ICON_DARK: &[u8] = include_bytes!("../icons/tray/icon-dark.png");
static ICON_LIGHT: &[u8] = include_bytes!("../icons/tray/icon-light.png");

static CURRENT_THEME: AtomicU8 = AtomicU8::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    Light = 0,
    Dark = 1,
}

impl Default for Theme {
    fn default() -> Self {
        Theme::Light
    }
}

impl From<u8> for Theme {
    fn from(value: u8) -> Self {
        match value {
            1 => Theme::Dark,
            _ => Theme::Light,
        }
    }
}

pub fn get_icon_for_theme(theme: Theme) -> &'static [u8] {
    match theme {
        Theme::Dark => ICON_DARK,
        Theme::Light => ICON_LIGHT,
    }
}

#[cfg(target_os = "macos")]
pub fn get_system_theme() -> Theme {
    use cocoa::appkit::NSApplication;
    use cocoa::base::nil;
    use cocoa::foundation::NSString;
    use objc::{msg_send, sel, sel_impl};

    unsafe {
        let app: cocoa::base::id = NSApplication::sharedApplication(nil);
        let appearance: cocoa::base::id = msg_send![app, effectiveAppearance];
        if appearance == nil {
            return Theme::default();
        }
        let name: cocoa::base::id = msg_send![appearance, name];
        if name == nil {
            return Theme::default();
        }
        let name_str = std::ffi::CStr::from_ptr(NSString::UTF8String(name))
            .to_str()
            .unwrap_or("");
        if name_str.contains("Dark") {
            Theme::Dark
        } else {
            Theme::Light
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub fn get_system_theme() -> Theme {
    Theme::default()
}

pub fn update_tray_icon(theme: Theme) {
    if let Ok(guard) = TRAY_ICON.lock() {
        if let Some(tray) = guard.as_ref() {
            let icon_bytes = get_icon_for_theme(theme);
            if let Ok(icon) = tauri::image::Image::from_bytes(icon_bytes) {
                let _ = tray.set_icon(Some(icon));
            }
        }
    }
}

#[cfg(target_os = "macos")]
pub fn start_theme_monitor(_app_handle: AppHandle) {
    CURRENT_THEME.store(get_system_theme() as u8, Ordering::SeqCst);

    std::thread::spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_millis(500));

            let new_theme = get_system_theme();
            let old_theme = Theme::from(CURRENT_THEME.load(Ordering::SeqCst));

            if new_theme != old_theme {
                CURRENT_THEME.store(new_theme as u8, Ordering::SeqCst);
                update_tray_icon(new_theme);
            }
        }
    });
}

#[cfg(not(target_os = "macos"))]
pub fn start_theme_monitor(_app_handle: AppHandle) {
}
