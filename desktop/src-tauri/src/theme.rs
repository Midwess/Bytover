/// Tray icon bytes - uses template image format on macOS.
/// On macOS, the OS automatically adjusts the color for light/dark mode.
/// The icon should be monochrome (black with alpha transparency).
pub static TRAY_ICON_BYTES: &[u8] = include_bytes!("../icons/tray/icon.png");
