use serde::{Deserialize, Serialize};
use surreal_derive_plus::SurrealDerive;

use super::transfer_resource::TransferResource;

#[derive(Debug, thiserror::Error)]
pub enum TransferProgressErrors {
    #[error("This resource already completed")]
    AlreadyCommitted
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, SurrealDerive)]
pub enum TransferProgressStatus {
    InProgress(f32),
    Success,
    Failed(String)
}

#[derive(Debug, Clone, Serialize, Deserialize, SurrealDerive)]
pub struct TransferProgress {
    resource_id: u64,
    resource_size: u64,
    transfered_amount: u64,
    status: TransferProgressStatus
}

impl PartialEq for TransferProgress {
    fn eq(&self, other: &Self) -> bool {
        other.resource_id() == self.resource_id()
    }
}

impl TransferProgress {
    pub fn new(resource: &TransferResource) -> Self {
        Self {
            resource_id: resource.order_id(),
            status: TransferProgressStatus::InProgress(0f32),
            transfered_amount: 0,
            resource_size: resource.size_in_bytes()
        }
    }

    pub fn resource_id(&self) -> u64 {
        self.resource_id
    }

    pub fn size(&self) -> u64 {
        self.resource_size
    }

    pub fn completion(&self) -> f32 {
        match &self.status {
            TransferProgressStatus::InProgress(progress) => *progress,
            TransferProgressStatus::Success => 1f32,
            TransferProgressStatus::Failed(_) => 1f32
        }
    }

    pub fn error_message(&self) -> Option<&str> {
        match &self.status {
            TransferProgressStatus::Failed(message) => Some(message),
            _ => None
        }
    }

    pub fn transfered_amount(&self) -> u64 {
        self.transfered_amount
    }

    pub fn status(&self) -> &TransferProgressStatus {
        &self.status
    }

    pub fn cancel(&mut self) {
        if matches!(self.status, TransferProgressStatus::InProgress(_)) {
            self.status = TransferProgressStatus::Failed("Canceled".to_owned());
        }
    }

    pub fn update_transfered_bytes(&mut self, new_amount: u64) -> Result<(), TransferProgressErrors> {
        if !matches!(self.status, TransferProgressStatus::InProgress(_)) {
            return Err(TransferProgressErrors::AlreadyCommitted)
        }

        self.transfered_amount = new_amount.min(self.resource_size);
        self.status = TransferProgressStatus::InProgress(self.transfered_amount as f32 / self.resource_size as f32);

        Ok(())
    }

    pub fn commit(&mut self, status: TransferProgressStatus) -> Result<(), TransferProgressErrors> {
        if matches!(status, TransferProgressStatus::InProgress(_)) {
            // Doing nothing
            log::warn!("Cannot commit an in-progress status");
            return Ok(())
        }

        if !matches!(self.status, TransferProgressStatus::InProgress(_)) {
            return Err(TransferProgressErrors::AlreadyCommitted)
        }

        self.status = status;

        match self.status {
            TransferProgressStatus::Success => self.transfered_amount = self.resource_size,
            _ => ()
        }

        Ok(())
    }
}
