use serde::{Deserialize, Serialize};
use uniffi::Record;

use crate::app::file_system::file::LocalResourcePath;

use super::avatar::AvatarViewModel;

#[derive(Clone, Record, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiveResourceViewModel {
    pub id: u64,
    pub name: String,
    pub display_size: String,
    pub thumbnail: Option<LocalResourcePath>,
    pub is_completed: bool
}

#[derive(Clone, Record, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReceiveSessionViewModel {
    pub id: u64,
    pub peer_avatar: AvatarViewModel,
    pub peer_name: String,
    pub peer_description: String,
    pub resources: Vec<ReceiveResourceViewModel>,
    pub is_completed: bool,
    pub is_in_progress: bool,
    pub display_download_speed: String,
    pub progress: f64
}
