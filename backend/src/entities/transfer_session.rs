use devlog_sdk::distributed_id::gen_id;
use schema::value::static_resource::StaticResource;
use serde::{Deserialize, Serialize};
use surreal_derive_plus::SurrealDerive;

use crate::entities::transfer_progress::{TransferProgress, TransferProgressStatus};
use crate::entities::transfer_resource::TransferResource;

use super::transfer_progress::TransferProgressErrors;
use super::transfer_resource::TransferResourceType;

#[derive(Debug, thiserror::Error)]
pub enum TransferSessionErrors {
    #[error("This resource is already exist {0}")]
    DuplicatedResource(u64),
    #[error("Resource not found")]
    ResourceNotFound,
    #[error("Transfer error {0}")]
    TransferProgressError(#[from] TransferProgressErrors),
    #[error("Max resources exceed {0}")]
    MaxResourceExceed(usize)
}

#[derive(Debug, Clone, Serialize, Deserialize, SurrealDerive)]
pub struct TransferSession {
    order_id: u64,
    owner_user_order_id: u64,
    alias: String,
    password: Option<String>,
    to_emails: Vec<String>,
    resources: Vec<TransferResource>,
    progress: Vec<TransferProgress>
}

impl TransferSession {
    pub async fn public(password: Option<String>, from_user: u64, alias: String, to_emails: Vec<String>) -> Self {
        Self {
            order_id: gen_id().await,
            owner_user_order_id: from_user,
            password,
            resources: Default::default(),
            progress: Default::default(),
            alias,
            to_emails
        }
    }

    pub fn to_emails(&self) -> &Vec<String> {
        self.to_emails.as_ref()
    }

    pub async fn start_transfer(
        &mut self,
        order_id: Option<u64>,
        name: impl Into<String>,
        size: u64,
        r#type: TransferResourceType
    ) -> Result<(), TransferSessionErrors> {
        if self.resources.len() > 2048 {
            return Err(TransferSessionErrors::MaxResourceExceed(2048))
        }

        let resource = TransferResource::new(order_id, self.order_id(), name, size, r#type).await;
        if self.resources.iter().any(|it| it.order_id() == resource.order_id()) {
            return Err(TransferSessionErrors::DuplicatedResource(resource.order_id()))
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
        let in_progress_resource_id = self
            .progress
            .iter()
            .find(|it| matches!(it.status(), TransferProgressStatus::InProgress(_)))
            .map(|it| it.resource_id())?;

        let in_progress_resource = self.resources.iter().find(|resource| resource.order_id() == in_progress_resource_id);

        in_progress_resource
    }

    pub fn current_resource_mut(&mut self) -> Option<&mut TransferResource> {
        let Some(current_id) = self.current_resource().map(|it| it.order_id()) else {
            return None;
        };

        self.resources.iter_mut().find(|it| it.order_id() == current_id)
    }

    pub fn into_resource(mut self, resource_id: u64) -> Option<TransferResource> {
        let Some(position) = self.resources.iter().position(|it| it.order_id() == resource_id) else {
            return None;
        };

        let resource = self.resources.swap_remove(position);
        Some(resource)
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
        let is_all_completed = self
            .progress
            .iter()
            .all(|it| matches!(it.status(), TransferProgressStatus::Success | TransferProgressStatus::Failed(_)));
        is_all_completed
    }

    pub fn user_order_id(&self) -> u64 {
        self.owner_user_order_id
    }

    pub fn order_id(&self) -> u64 {
        self.order_id
    }

    pub fn resources(&self) -> &Vec<TransferResource> {
        &self.resources
    }

    pub fn progresses(&self) -> &Vec<TransferProgress> {
        &self.progress
    }

    pub fn update_transferred_progress(&mut self, resource_id: u64, transferred_size: u64) {
        let Some(progress) = self.progress.iter_mut().find(|it| it.resource_id() == resource_id) else {
            return;
        };

        let _ = progress.update_transfered_bytes(transferred_size);
    }

    pub fn access_url(&self, base_url: String) -> String {
        format!("{base_url}?session={}", self.alias)
    }

    pub fn password(&self) -> Option<String> {
        self.password.clone()
    }

    pub fn validate_access(&self, entered_password: Option<String>) -> bool {
        if let Some(password) = &self.password {
            let Some(entered_password) = entered_password else { return false };

            return entered_password.eq(password);
        };

        true
    }

    pub fn thumbnail_resources(&self) -> Vec<(u64, StaticResource)> {
        self.resources
            .iter()
            .filter_map(|it| it.thumbnail_source().map(|it2| (it.order_id(), it2)))
            .collect::<Vec<_>>()
    }
}
