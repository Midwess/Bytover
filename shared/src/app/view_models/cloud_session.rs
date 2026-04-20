use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CloudSession {
    pub shelf_id: Option<String>,
    pub access_url: Option<String>,
    pub password: Option<String>,
    pub session_id: String,
    pub is_completed: bool,
    pub is_in_progress: bool,
    pub display_download_speed: String,
    pub progress: f64,
    pub is_email: bool,
}

impl CloudSession {}
