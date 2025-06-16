use core_services::db::repository::abstraction::errors::RepositoryError;
use core_services::db::repository::abstraction::repository::Repository;

use crate::cloud_storage::storage::CloudStorage;
use crate::entities::transfer_progress::{TransferProgressErrors, TransferProgressStatus};
use crate::entities::transfer_resource::TransferResource;
use crate::entities::transfer_session::{TransferSession, TransferSessionErrors};
use crate::repositories::transfer_session::{TransferSessionId, TransferSessionRepository};

#[derive(Debug, thiserror::Error)]
enum TransferErrors {
    #[error("Session not found")]
    SessionNotFound,
    #[error("Resource not found or already completed")]
    ResourceNotFoundOrAlreadyCompleted,
    #[error("Session error {0}")]
    SessionError(#[from] TransferSessionErrors),
    #[error("Empty resources")]
    EmptyResources,
    #[error("System error {0}")]
    SystemError(#[from] RepositoryError),
    #[error("Upload error {0}")]
    TransferProgressError(#[from] TransferProgressErrors)
}

pub struct TransferService {
    pub transfer_repository: Box<dyn TransferSessionRepository>,
    pub cloud_storage: Box<dyn CloudStorage>
}

impl TransferService {
    pub async fn start_public_transfer(
        &self,
        user_id: u64,
        password: Option<String>,
        resources: Vec<TransferResource>
    ) -> Result<TransferSession, TransferErrors> {
        if resources.is_empty() {
            return Err(TransferErrors::EmptyResources)
        }

        let mut session = TransferSession::public(password, user_id).await;

        for resource in resources {
            session.start_transfer(resource)?;
        }

        let session = self.transfer_repository.create(session).await?;

        Ok(session)
    }

    pub async fn cancel_transfer(&self, user_id: u64, session_order_id: u64) -> Result<(), TransferErrors> {
        let session_id = TransferSessionId {
            order_id: Some(session_order_id),
            user_order_id: Some(user_id)
        };

        let Some(mut session) = self.transfer_repository.find_one(&session_id).await? else {
            return Err(TransferErrors::SessionNotFound)
        };

        session.cancel();

        let _ = self.transfer_repository.update_one(session).await?;

        Ok(())
    }

    pub async fn commit_resource(
        &self,
        user_id: u64,
        session_order_id: u64,
        resource_id: u64,
        status: TransferProgressStatus
    ) -> Result<Option<TransferResource>, TransferErrors> {
        let session_id = TransferSessionId {
            order_id: Some(session_order_id),
            user_order_id: Some(user_id)
        };

        let Some(mut session) = self.transfer_repository.find_one(&session_id).await? else {
            return Err(TransferErrors::SessionNotFound)
        };

        let Some(current_progress) = session.current_resource_progress_mut() else {
            return Err(TransferErrors::ResourceNotFoundOrAlreadyCompleted)
        };

        if current_progress.resource_id() != resource_id {
            log::warn!("Resource already completed session id: {session_order_id}; resource id: {resource_id}");
            return Err(TransferErrors::ResourceNotFoundOrAlreadyCompleted)
        }

        current_progress.commit(status)?;

        let updated_session = self.transfer_repository.update_one(session).await?;

        Ok(updated_session.current_resource().cloned())
    }
}
