use super::selected_resource::SelectedResourceViewModel;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReceiveResourceViewModel {
    pub model: SelectedResourceViewModel,
    pub completion: f32,
    pub is_completed: bool,
    pub is_ready: bool,
    pub is_success: bool
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReceiveSessionViewModel {
    pub id: String,
    pub is_cloud: bool,
    pub is_scope_online: bool,
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
    pub loading_status: Option<String>,
    pub error_message: Option<String>,
    pub resources: Vec<ReceiveResourceViewModel>,
    pub is_completed: bool,
    pub is_in_progress: bool,
    pub display_download_speed: String,
    pub progress: f64,
    pub display_datetime: String,
    pub download_all_resource: Option<ReceiveResourceViewModel>
}
