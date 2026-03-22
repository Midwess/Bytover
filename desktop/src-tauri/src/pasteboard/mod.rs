#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "windows")]
mod windows;

use shared::app::shelf::module::ResourceSelection;

pub async fn read_drag_pasteboard_selections() -> Result<Vec<ResourceSelection>, String> {
    #[cfg(target_os = "macos")]
    {
        macos::read_drag_pasteboard_selections().await
    }
    #[cfg(target_os = "windows")]
    {
        windows::read_drag_pasteboard_selections().await
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Ok(vec![])
    }
}
