use serde::{Deserialize, Serialize};
use uniffi::Record;

use super::avatar::AvatarViewModel;
use super::selected_resource::SelectedResourceViewModel;

#[derive(Clone, Record, Debug, PartialEq, Serialize, Deserialize)]
pub struct ImageReceiveResourceViewModel {
    pub model: SelectedResourceViewModel,
    pub is_completed: bool
}

#[derive(Clone, Record, Debug, PartialEq, Serialize, Deserialize)]
pub struct VideoReceiveResourceViewModel {
    pub model: SelectedResourceViewModel,
    pub is_completed: bool
}

#[derive(Clone, Record, Debug, PartialEq, Serialize, Deserialize)]
pub struct FileReceiveResourceViewModel {
    pub model: SelectedResourceViewModel,
    pub is_completed: bool
}

#[derive(Clone, Record, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReceiveSessionViewModel {
    pub id: u64,
    pub peer_avatar: AvatarViewModel,
    pub peer_name: String,
    pub peer_description: String,
    pub image_resources: Vec<ImageReceiveResourceViewModel>,
    pub video_resources: Vec<VideoReceiveResourceViewModel>,
    pub file_resources: Vec<FileReceiveResourceViewModel>,
    pub is_completed: bool,
    pub is_in_progress: bool,
    pub display_download_speed: String,
    pub progress: f64
}
