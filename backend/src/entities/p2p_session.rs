use devlog_sdk::distributed_id::gen_id;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct P2PSession {
    session_id: u64,
    device_id: u64,
    user_id: u64,
    alias: String,
    description: Option<String>
}

impl P2PSession {
    pub async fn new(device_id: u64, user_id: u64, alias: String, description: Option<String>) -> Self {
        Self {
            session_id: gen_id().await,
            device_id,
            user_id,
            alias,
            description
        }
    }

    pub fn from_db(session_id: u64, device_id: u64, user_id: u64, alias: String, description: Option<String>) -> Self {
        Self {
            session_id,
            device_id,
            user_id,
            alias,
            description
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
        format!("{base_url}/transfer?session={}", self.alias)
    }

    pub fn get_scope(&self) -> String {
        format!("{}-{}", self.alias, self.session_id)
    }

    pub fn owner_signalling_key(&self) -> String {
        format!("direct://{};owner", self.get_scope())
    }

    pub fn member_signalling_key(&self) -> String {
        format!("direct://{};member", self.get_scope())
    }
}
