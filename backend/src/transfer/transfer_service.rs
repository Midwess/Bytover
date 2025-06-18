use core_services::db::repository::abstraction::errors::RepositoryError;
use schema::value::static_resource::StaticResource;

use crate::cloud_storage::storage::{CloudStorage, CloudStorageErrors};
use crate::entities::transfer_progress::{TransferProgressErrors, TransferProgressStatus};
use crate::entities::transfer_resource::{TransferResource, TransferResourceType};
use crate::entities::transfer_session::{TransferSession, TransferSessionErrors};
use crate::repositories::transfer_session::{TransferSessionId, TransferSessionRepository};

#[derive(Debug, thiserror::Error)]
pub enum TransferErrors {
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
    TransferProgressError(#[from] TransferProgressErrors),
    #[error("Cloud storage error {0}")]
    CloudStroageError(#[from] CloudStorageErrors)
}

pub struct StartTransferResourceRequest {
    // The user can decide the order id
    // it cannot be duplicated
    pub order_id: Option<u64>,
    pub name: String,
    pub r#type: TransferResourceType,
    pub size: u64
}

pub struct StartTransferResourceResponse {
    pub session_id: u64,
    pub first_resource: TransferResource,
    pub thumbnails: Vec<(u64, StaticResource)>
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
        requests: Vec<StartTransferResourceRequest>
    ) -> Result<StartTransferResourceResponse, TransferErrors> {
        if requests.is_empty() {
            return Err(TransferErrors::EmptyResources)
        }

        let mut session = TransferSession::public(password, user_id).await;

        for request in requests {
            session.start_transfer(request.order_id, request.name, request.size, request.r#type).await?;
        }

        let session = self.transfer_repository.create(session).await?;

        let order_id = session.order_id();
        let Some(first_resource_id) = session.current_resource().map(|it| it.order_id()) else {
            log::warn!("The first resource must be defined, session id = {}", session.order_id());
            return Err(TransferErrors::ResourceNotFoundOrAlreadyCompleted)
        };

        let mut thumbnails = session.thumbnail_resources();

        for thumbnail in thumbnails.iter_mut() {
            let _ = self.cloud_storage.sign(&mut thumbnail.1).await;
        }

        let Some(first_resource) = session.into_resource(first_resource_id) else {
            return Err(TransferErrors::ResourceNotFoundOrAlreadyCompleted)
        };

        let response = StartTransferResourceResponse {
            session_id: order_id,
            first_resource,
            thumbnails
        };

        Ok(response)
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

        let session = self.transfer_repository.update_one(session).await?;

        let Some(next_resource_id) = session.current_resource().map(|it| it.order_id()) else {
            return Ok(None)
        };

        let Some(next_resource) = session.into_resource(next_resource_id) else {
            return Ok(None)
        };

        Ok(Some(next_resource))
    }
}
