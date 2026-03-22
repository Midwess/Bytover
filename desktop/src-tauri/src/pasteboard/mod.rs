#[cfg(target_os = "macos")]
mod macos;

use shared::app::shelf::module::ResourceSelection;

pub async fn read_drag_pasteboard_selections() -> Result<Vec<ResourceSelection>, String> {
    #[cfg(target_os = "macos")]
    {
        macos::read_drag_pasteboard_selections().await
    }
    #[cfg(not(target_os = "macos"))]
    {
        Ok(vec![])
    }
}
