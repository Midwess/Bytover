use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceAlias {
    alias: String,
    user_id: u64,
    device_id: u64,
}

impl DeviceAlias {
    pub fn new(alias: String, user_id: u64, device_id: u64) -> Self {
        Self {
            alias,
            user_id,
            device_id,
        }
    }

    pub fn from_db(alias: String, user_id: u64, device_id: u64) -> Self {
        Self {
            alias,
            user_id,
            device_id,
        }
    }

    pub fn alias(&self) -> &str {
        &self.alias
    }

    pub fn user_id(&self) -> u64 {
        self.user_id
    }

    pub fn device_id(&self) -> u64 {
        self.device_id
    }
}
