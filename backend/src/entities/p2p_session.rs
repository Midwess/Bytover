use devlog_sdk::distributed_id::gen_id;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct P2PSession {
    pub session_id: u64,
    pub device_id: u64,
    pub user_id: u64,
    pub alias: String,
    pub description: Option<String>,
    pub signalling_key: String,
}

impl P2PSession {
    pub async fn new(device_id: u64, user_id: u64, alias: String, description: Option<String>, signalling_key: String) -> Self {
        Self {
            session_id: gen_id().await,
            device_id,
            user_id,
            alias,
            description,
            signalling_key,
        }
    }

    pub fn from_db(
        session_id: u64,
        device_id: u64,
        user_id: u64,
        alias: String,
        description: Option<String>,
        signalling_key: String,
    ) -> Self {
        Self {
            session_id,
            device_id,
            user_id,
            alias,
            description,
            signalling_key,
        }
    }

    pub fn session_id(&self) -> u64 {
        self.session_id
    }

    pub fn device_id(&self) -> u64 {
        self.device_id
    }

    pub fn user_id(&self) -> u64 {
        self.user_id
    }

    pub fn alias(&self) -> &str {
        &self.alias
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    pub fn access_url(&self, base_url: String) -> String {
        format!("{base_url}/session/{}", self.alias)
    }

    pub fn signalling_key(&self) -> &str {
        &self.signalling_key
    }
}
