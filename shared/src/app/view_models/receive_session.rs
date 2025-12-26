use serde::{Deserialize, Serialize};

use super::avatar::AvatarViewModel;
use super::selected_resource::SelectedResourceViewModel;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ImageReceiveResourceViewModel {
    pub model: SelectedResourceViewModel,
    pub completion: f32,
    pub is_completed: bool
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VideoReceiveResourceViewModel {
    pub model: SelectedResourceViewModel,
    pub completion: f32,
    pub is_completed: bool
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FileReceiveResourceViewModel {
    pub model: SelectedResourceViewModel,
    pub completion: f32,
    pub is_completed: bool
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReceiveSessionViewModel {
    pub id: String,
    pub peer_id: String,
    pub peer_avatar: AvatarViewModel,
    pub peer_name: String,
    pub peer_description: String,
    pub password_required: bool,
    pub is_authenticated: bool,
    pub has_details: bool,
    pub image_resources: Vec<ImageReceiveResourceViewModel>,
    pub video_resources: Vec<VideoReceiveResourceViewModel>,
    pub file_resources: Vec<FileReceiveResourceViewModel>,
    pub is_completed: bool,
    pub is_in_progress: bool,
    pub display_download_speed: String,
    pub progress: f64,
    pub display_datetime: String
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReceiveCloudSessionViewModel {
    pub id: String,
    pub is_loading: bool,
    pub password: Option<String>,
    pub avatar_url: String,
    pub sender_name: String,
    pub image_resources: Vec<ImageReceiveResourceViewModel>,
    pub video_resources: Vec<VideoReceiveResourceViewModel>,
    pub file_resources: Vec<FileReceiveResourceViewModel>,
    pub display_datetime: String,
    pub access_url: String,
    pub is_required_password: bool,
    pub alias: Option<String>
}
