use devlog_sdk::distributed_id::gen_id;
use serde::{Deserialize, Serialize};
use surreal_derive_plus::SurrealDerive;

use crate::entities::transfer_progress::{TransferProgress, TransferProgressStatus};
use crate::entities::transfer_resource::TransferResource;

use super::transfer_progress::TransferProgressErrors;

#[derive(Debug, thiserror::Error)]
pub enum TransferSessionErrors {
    #[error("This resource is already transfered {0}")]
    DuplicatedResource(String),
    #[error("Resource not found")]
    ResourceNotFound,
    #[error("Transfer error {0}")]
    TransferProgressError(#[from] TransferProgressErrors)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, SurrealDerive)]
pub struct TransferSession {
    id: u64,
    owner_user_order_id: u64,
    password: Option<String>,
    resources: Vec<TransferResource>,
    progress: Vec<TransferProgress>
}

impl TransferSession {
    pub async fn public(password: Option<String>, from_user: u64) -> Self {
        Self {
            id: gen_id().await,
            owner_user_order_id: from_user,
            password,
            resources: Default::default(),
            progress: Default::default()
        }
    }

    pub fn start_transfer(&mut self, resource: TransferResource) -> Result<(), TransferSessionErrors> {
        if self.resources.iter().any(|it| it.order_id() == resource.order_id()) {
            return Err(TransferSessionErrors::DuplicatedResource(resource.name().to_owned()))
        }

        self.progress.push(TransferProgress::new(&resource));
        self.resources.push(resource);
        let size_desc = |a: &TransferResource, b: &TransferResource| b.size_in_bytes().cmp(&a.size_in_bytes());
        self.resources.sort_by(size_desc);
        Ok(())
    }

    pub fn current_resource_progress_mut(&mut self) -> Option<&mut TransferProgress> {
        let Some(current_resource_id) = self.current_resource().map(|it| it.order_id()) else {
            return None;
        };

        self.progress.iter_mut().find(|it| it.resource_id() == current_resource_id)
    }

    pub fn current_resource(&self) -> Option<&TransferResource> {
        let in_progress_resource = self.resources.iter().find(|resource| {
            let Some(progress) = self.progress.iter().find(|progress| progress.resource_id() == resource.order_id()) else {
                return false;
            };

            if matches!(progress.status(), TransferProgressStatus::InProgress(0f32)) {
                return true
            }

            false
        });

        in_progress_resource
    }

    pub fn cancel(&mut self) {
        self.progress
            .iter_mut()
            .filter(|it| matches!(it.status(), TransferProgressStatus::InProgress(_)))
            .for_each(|it| it.cancel());
    }

    pub fn commit_resource(
        &mut self,
        resource_id: u64,
        transfer_status: TransferProgressStatus
    ) -> Result<Option<&TransferResource>, TransferSessionErrors> {
        let Some(progress) = self.progress.iter_mut().find(|it| it.resource_id() == resource_id) else {
            return Err(TransferSessionErrors::ResourceNotFound)
        };

        progress.commit(transfer_status)?;

        Ok(self.current_resource())
    }

    pub fn is_completed(&self) -> bool {
        let is_all_completd = self
            .progress
            .iter()
            .all(|it| matches!(it.status(), TransferProgressStatus::Success | TransferProgressStatus::Failed(_)));
        is_all_completd
    }

    pub fn user_order_id(&self) -> u64 {
        self.owner_user_order_id
    }

    pub fn order_id(&self) -> u64 {
        self.id
    }
}
