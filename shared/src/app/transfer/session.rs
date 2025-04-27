use std::time::Instant;

use chrono::{Date, DateTime, Utc};
use serde::{Deserialize, Serialize};
use surreal_derive_plus::SurrealDerive;
use uniffi::{Enum, Record};
use tokio::time::Duration;

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

#[derive(Debug, PartialEq, Record, Serialize, Deserialize, Clone, SurrealDerive)]
pub struct TransferProgress {
    pub resource_order_id: u64,
    pub file_size: u64,
    pub total_bytes_counter: u64,
    pub bytes_per_second: u64,
    pub start_time_utc_ms: u64,
    pub bytes_sec_counter: u64,
    pub transfer_type: TransferType,
    pub status: TransferStatus
}

impl TransferProgress {
    pub fn new(resource_order_id: u64, file_size: u64, transfer_type: TransferType) -> Self {
        Self {
            resource_order_id,
            file_size,
            total_bytes_counter: 0,
            bytes_per_second: 0,
            bytes_sec_counter: 0,
            start_time_utc_ms: Utc::now().timestamp_millis() as u64,
            transfer_type,
            status: TransferStatus::InProgress
        }
    }

    pub fn complete(&mut self) {
        self.status = if self.percentage() == 1.0 {
            TransferStatus::Success
        } else {
            TransferStatus::Fail(format!(
                "Data corrupted transfer for resource {} received {}/1.0",
                self.resource_order_id, self.percentage()
            ))
        };

        self.total_bytes_counter = self.file_size;
        self.bytes_per_second = 0;
        self.bytes_sec_counter = 0;
    }

    pub fn success(&mut self) {
        self.complete();
        self.status = TransferStatus::Success;
    }

    pub fn fail(&mut self, msg: String) {
        self.complete();
        if self.percentage() == 1.0 {
            self.success();
        } else {
            self.status = TransferStatus::Fail(msg);
        }
    }

    pub fn percentage(&self) -> f64 {
        (self.total_bytes_counter as f64 / self.file_size as f64).min(1.0)
    }

    pub fn is_completed(&self) -> bool {
        self.percentage() == 1.0
    }

    pub fn elapsed(&self) -> u64 {
        Utc::now().timestamp_millis() as u64 - self.start_time_utc_ms
    }

    pub fn update_progress(&mut self, bytes_count: u64) {
        let elapsed = self.elapsed(); 

        self.total_bytes_counter += bytes_count;

        self.bytes_sec_counter += bytes_count;

        if elapsed >= 1000 {
            self.bytes_per_second = (self.bytes_sec_counter as f64 / (elapsed.max(1) as f64 / 1000.0)).round() as u64;
            self.start_time_utc_ms = Utc::now().timestamp_millis() as u64;
            self.bytes_sec_counter = bytes_count;
        }

        if self.bytes_per_second == 0 {
            self.bytes_per_second = (self.bytes_sec_counter as f64 / (elapsed.max(1) as f64 / 1000.0)).round() as u64;
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

    pub fn is_initializing(&self) -> bool {
        self.progress.iter().all(|it| it.status == TransferStatus::InProgress && it.bytes_per_second == 0)
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
        let total_size = self.resources.iter().map(|it| it.size).sum::<u64>();
        if total_size == 0 {
            return 1.0;
        }

        let total_bytes_sent = self.progress.iter().map(|it| it.total_bytes_counter).sum::<u64>();
        log::info!(target: "tiendang-debug", "Total progress: {} / {} speed = {}", total_bytes_sent, total_size, self.bytes_per_second());
        total_bytes_sent as f64 / total_size as f64
    }

    pub fn bytes_per_second(&self) -> u64 {
        self.progress.iter().map(|it| it.bytes_per_second).sum::<u64>()
    }

    pub fn is_completed(&self) -> bool {
        let resource_left = self.progress.iter().filter(|it| !it.status.is_completed()).count();

        resource_left == 0
    }
}
