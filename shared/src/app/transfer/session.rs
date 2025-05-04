use std::fmt::Display;

use chrono::Utc;
use devlog_sdk::distributed_id::gen_id;
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

#[derive(Debug, PartialEq, Enum, Serialize, Deserialize, Clone, SurrealDerive)]
pub enum TransferSessionStatus {
    Initializing,
    InProgress { bytes_per_second: u64, percentage: f64 },
    Success,
    Failed(String),
    Canceled
}

impl Display for TransferSessionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransferSessionStatus::Initializing => write!(f, "Initializing..."),
            TransferSessionStatus::InProgress { bytes_per_second, .. } => {
                let kb_per_second = *bytes_per_second as f64 / 1024.0;
                if kb_per_second < 100.0 {
                    write!(f, "{:.1} KB/s", kb_per_second)
                } else {
                    write!(f, "{:.1} MB/s", kb_per_second / 1024.0)
                }
            }
            TransferSessionStatus::Success => write!(f, "Success"),
            TransferSessionStatus::Failed(msg) => write!(f, "Failed {}", msg),
            TransferSessionStatus::Canceled => write!(f, "Canceled")
        }
    }
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
    bytes_per_second: u64,
    start_time_utc_ms: u64,
    bytes_sec_counter: u64,
    last_update_time_ms: u64,
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
            last_update_time_ms: Utc::now().timestamp_millis() as u64,
            start_time_utc_ms: Utc::now().timestamp_millis() as u64,
            transfer_type,
            status: TransferStatus::Pending
        }
    }

    pub fn complete(&mut self) {
        self.status = if self.percentage() == 1.0 {
            TransferStatus::Success
        } else {
            TransferStatus::Fail(format!(
                "Data corrupted transfer for resource {} received {}/1.0",
                self.resource_order_id,
                self.percentage()
            ))
        };
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
        self.is_failed() || self.is_success() || self.is_canceled()
    }

    pub fn is_failed(&self) -> bool {
        matches!(self.status, TransferStatus::Fail(_))
    }

    pub fn is_success(&self) -> bool {
        matches!(self.status, TransferStatus::Success)
    }

    pub fn is_canceled(&self) -> bool {
        matches!(self.status, TransferStatus::Canceled)
    }

    pub fn elapsed(&self) -> u64 {
        Utc::now().timestamp_millis() as u64 - self.start_time_utc_ms
    }

    pub fn update_progress(&mut self, bytes_count: u64) {
        if bytes_count > 0 {
            self.last_update_time_ms = Utc::now().timestamp_millis() as u64;
        }

        if self.status == TransferStatus::Pending {
            self.status = TransferStatus::InProgress;
        }

        if self.status != TransferStatus::InProgress {
            return;
        }

        let elapsed = self.elapsed();

        self.total_bytes_counter += bytes_count;

        self.bytes_sec_counter += bytes_count;

        if elapsed >= 1000 {
            self.start_time_utc_ms = Utc::now().timestamp_millis() as u64;
            self.bytes_sec_counter = bytes_count;
        }

        self.bytes_per_second = (self.bytes_sec_counter as f64 / (elapsed.max(1) as f64 / 1000.0)).round() as u64;

        if self.percentage() == 1.0 {
            self.success();
        }
    }

    pub fn bytes_per_second(&self) -> u64 {
        if self.last_update_time_ms < Utc::now().timestamp_millis() as u64 - 2000 {
            return 0;
        }

        self.bytes_per_second
    }
}

#[derive(Debug, PartialEq, Enum, Serialize, Deserialize, Clone, SurrealDerive)]
pub enum TransferStatus {
    Pending,
    InProgress,
    Fail(String),
    Success,
    Canceled
}

impl TransferStatus {
    pub fn is_completed(&self) -> bool {
        matches!(
            self,
            TransferStatus::Success | TransferStatus::Fail(_) | TransferStatus::Canceled
        )
    }
}

impl TransferSession {
    pub async fn answer(id: u64, mut out_resources: Vec<LocalResource>, target: TransferTarget) -> Self {
        out_resources.sort_by(|a, b| a.size.cmp(&b.size));
        Self {
            order_id: id,
            progress: out_resources
                .iter()
                .map(|it| TransferProgress::new(it.order_id, it.size, TransferType::Receive))
                .collect(),
            resources: out_resources,
            transfer_type: TransferType::Receive,
            target
        }
    }

    pub async fn send(resources: Vec<LocalResource>, target: TransferTarget) -> Self {
        let mut resources = resources;
        resources.sort_by(|a, b| a.size.cmp(&b.size));
        Self {
            order_id: gen_id().await,
            progress: resources.iter().map(|it| TransferProgress::new(it.order_id, it.size, TransferType::Send)).collect(),
            resources,
            transfer_type: TransferType::Send,
            target
        }
    }

    pub fn add_resource(&mut self, resource: LocalResource) {
        self.resources.push(resource);
        self.resources.sort_by(|a, b| a.size.cmp(&b.size));
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
        total_bytes_sent as f64 / total_size as f64
    }

    pub fn bytes_per_second(&self) -> u64 {
        self.progress.iter().map(|it| it.bytes_per_second()).sum::<u64>()
    }

    pub fn is_completed(&self) -> bool {
        let resource_left = self.progress.iter().filter(|it| !it.status.is_completed()).count();

        resource_left == 0
    }

    pub fn cancel(&mut self) {
        self.progress.iter_mut().for_each(|it| {
            if it.status == TransferStatus::InProgress || it.status == TransferStatus::Pending {
                it.status = TransferStatus::Canceled;
            }
        });
    }

    pub fn get_next_download_resource(&self) -> Option<&LocalResource> {
        if self.transfer_type == TransferType::Send {
            return None;
        }

        self.resources.iter().find(|resource| {
            self.progress
                .iter()
                .find(|it| it.resource_order_id == resource.order_id)
                .expect("Resource missing progress")
                .status ==
                TransferStatus::Pending
        })
    }

    pub fn status(&self) -> TransferSessionStatus {
        if self.is_initializing() {
            return TransferSessionStatus::Initializing;
        }

        let is_canceled = self.progress.iter().any(|it| it.status == TransferStatus::Canceled);
        if is_canceled {
            return TransferSessionStatus::Canceled;
        }

        let is_in_progress = self
            .progress
            .iter()
            .any(|it| it.status == TransferStatus::InProgress || it.status == TransferStatus::Pending);
        if is_in_progress {
            return TransferSessionStatus::InProgress {
                bytes_per_second: self.bytes_per_second(),
                percentage: self.total_progress()
            }
        }

        let failed_messages = self
            .progress
            .iter()
            .filter_map(|it| match &it.status {
                TransferStatus::Fail(msg) => Some(msg.clone()),
                _ => None
            })
            .collect::<Vec<String>>();

        if !failed_messages.is_empty() {
            return TransferSessionStatus::Failed(failed_messages.join(", "));
        }

        TransferSessionStatus::Success
    }

    pub fn resource_progress(&self, resource_id: u64) -> Option<&TransferProgress> {
        self.progress.iter().find(|it| it.resource_order_id == resource_id)
    }

    pub fn resource_mut_progress(&mut self, resource_id: u64) -> Option<&mut TransferProgress> {
        self.progress.iter_mut().find(|it| it.resource_order_id == resource_id)
    }
}
