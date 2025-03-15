use serde::{Deserialize, Serialize};
use surreal_derive_plus::SurrealDerive;
use uniffi::{Enum, Record};

use crate::app::file_system::file::LocalResource;
use crate::entities::user::User;

#[derive(Debug, PartialEq, Record, Serialize, Deserialize, Clone, SurrealDerive)]
pub struct TransferSession {
    pub order_id: u64,
    pub resources: Vec<LocalResource>,
    pub progress: TransferProgress,
    pub target: TransferTarget
}

#[derive(Debug, PartialEq, Enum, Serialize, Deserialize, Clone, SurrealDerive)]
pub enum TransferTarget {
    User(User),
    Device(String),
    Internet(String)
}

#[derive(Debug, PartialEq, Serialize, Record, Deserialize, Clone, SurrealDerive)]
pub struct TransferProgress {
    pub percentage: f32,
    pub status: TransferStatus
}

#[derive(Debug, PartialEq, Enum, Serialize, Deserialize, Clone, SurrealDerive)]
pub enum TransferStatus {
    Fail,
    InProgress,
    Success
}

impl TransferSession {
    pub fn add_resource(&mut self, resource: LocalResource) {
        self.resources.push(resource);
    }
}
