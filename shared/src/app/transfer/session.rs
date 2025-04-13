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

    pub fn progress(resource_order_id: u64, percentage: f64) -> Self {
        Self {
            resource_order_id,
            percentage,
            status: if percentage == 1.0 {
                TransferStatus::Success
            } else {
                TransferStatus::InProgress
            }
        }
    }

    pub fn success(resource_order_id: u64) -> Self {
        Self {
            resource_order_id,
            percentage: 1.0,
            status: TransferStatus::Success
        }
    }

    pub fn fail(resource_order_id: u64, percentage: f64, msg: String) -> Self {
        if percentage == 1.0 {
            Self::success(resource_order_id)
        } else {
            Self {
                resource_order_id,
                percentage,
                status: TransferStatus::Fail(msg)
            }
        }
    }
}

#[derive(Debug, PartialEq, Enum, Serialize, Deserialize, Clone, SurrealDerive)]
pub enum TransferStatus {
    Fail(String),
    InProgress,
    Success
}

impl TransferStatus {
    pub fn is_completed(&self) -> bool {
        matches!(self, TransferStatus::Success | TransferStatus::Fail(_))
    }
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

    pub fn force_complete(&mut self, msg: String) {
        self.progress.iter_mut().for_each(|it| {
            it.status = TransferStatus::Fail(msg.clone());
        });
    }

    pub fn total_progress(&self) -> f64 {
        self.progress.iter().map(|it| it.percentage).sum::<f64>() / self.progress.len() as f64
    }

    pub fn is_completed(&self) -> bool {
        self.progress.iter().all(|it| it.status.is_completed())
    }
}
