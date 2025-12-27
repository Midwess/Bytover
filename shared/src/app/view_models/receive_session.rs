use serde::{Deserialize, Serialize};

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
    pub sender_id: String,
    pub sender_avatar: String,
    pub sender_name: String,
    pub sender_description: String,
    pub alias: Option<String>,
    pub access_url: Option<String>,
    pub password: Option<String>,
    pub password_required: bool,
    pub is_authenticated: bool,
    pub has_details: bool,
    pub is_loading: bool,
    pub image_resources: Vec<ImageReceiveResourceViewModel>,
    pub video_resources: Vec<VideoReceiveResourceViewModel>,
    pub file_resources: Vec<FileReceiveResourceViewModel>,
    pub is_completed: bool,
    pub is_in_progress: bool,
    pub display_download_speed: String,
    pub progress: f64,
    pub display_datetime: String
}
