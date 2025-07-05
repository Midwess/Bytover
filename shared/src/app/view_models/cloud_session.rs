use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CloudSession {
    pub access_url: Option<String>,
    pub password: Option<String>,
    pub session_id: u64,
    pub is_completed: bool,
    pub is_in_progress: bool,
    pub display_download_speed: String,
    pub progress: f64
}

impl CloudSession {}
