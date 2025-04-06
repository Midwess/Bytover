use serde::{Deserialize, Serialize};
use surreal_derive_plus::SurrealDerive;
use uniffi::{Enum, Record};

use crate::app::file_system::file::LocalResource;

use super::target::TransferTarget;

#[derive(Debug, PartialEq, Enum, Serialize, Deserialize, Clone, SurrealDerive)]
pub enum TransferType {
    Send,
    Receive
}

#[derive(Debug, PartialEq, Record, Serialize, Deserialize, Clone, SurrealDerive)]
pub struct TransferSession {
    pub order_id: u64,
    pub resources: Vec<LocalResource>,
    pub progress: Vec<TransferProgress>,
    pub transfer_type: TransferType,
    pub target: TransferTarget
}

#[derive(Debug, PartialEq, Serialize, Record, Deserialize, Clone, SurrealDerive)]
pub struct TransferProgress {
    pub resource_order_id: u64,
    pub percentage: f64,
    pub status: TransferStatus
}

impl TransferProgress {
    pub fn new(resource_order_id: u64) -> Self {
        Self {
            resource_order_id,
            percentage: 0.0,
            status: TransferStatus::InProgress
        }
    }
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

    pub fn peer_id(&self) -> Option<u128> {
        match &self.target {
            TransferTarget::Nearby(peer) => Some(peer.id()),
            _ => None
        }
    }

    pub fn update_progress(&mut self, progress: TransferProgress) {
        if let Some(index) = self.progress.iter().position(|it| it.resource_order_id == progress.resource_order_id) {
            self.progress[index] = progress;
        } else {
            self.progress.push(progress);
        }
    }

    pub fn total_progress(&self) -> f64 {
        self.progress.iter().map(|it| it.percentage).sum::<f64>() / self.progress.len() as f64
    }
}
