use crate::app_gateway::markov::{Markov, MarkovErrors};
use crate::cloud_storage::storage::{CloudStorage, CloudStorageErrors};
use crate::entities::transfer_progress::{TransferProgressErrors, TransferProgressStatus};
use crate::entities::transfer_resource::{TransferResource, TransferResourceType};
use crate::entities::transfer_session::{TransferSession, TransferSessionErrors};
use crate::mail::service::EmailService;
use crate::repositories::transfer_session::{TransferSessionId, TransferSessionRepository};
use core_services::db::repository::abstraction::errors::RepositoryError;
use schema::crafter::email_template::Template::{self, SendFile};
use schema::crafter::{EmailTemplate, SendFileTemplate};
use schema::devlog::auth_gateway::models::User;
use schema::value::static_resource::StaticResource;
use schema::value::datetime::Datetime;
use schema::crafter::FileResource as MailFileResource;

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
    CloudStorageError(#[from] CloudStorageErrors),
    #[error("Password of a session cannot exceed {0}")]
    PasswordLengthExceed(usize),
    #[error("Failed to generate alias {}", .0)]
    MarkovError(#[from] MarkovErrors)
}

pub struct TransferResourceRequest {
    // The user can decide the order id
    // it cannot be duplicated
    pub order_id: Option<u64>,
    pub name: String,
    pub r#type: TransferResourceType,
    pub size: u64
}

pub struct TransferResourcesResponse {
    pub session_id: u64,
    pub first_resource: TransferResource,
    pub thumbnails: Vec<(u64, StaticResource)>
}

pub struct TransferService {
    pub transfer_repository: Box<dyn TransferSessionRepository>,
    pub cloud_storage: Box<dyn CloudStorage>,
    pub markov_generator: Box<dyn Markov>,
    pub email_service: Box<dyn EmailService>
}

impl TransferService {
    pub async fn create_public_transfer_session(
        &self,
        user: &User,
        password: Option<String>,
        to_email: Option<String>,
    ) -> Result<TransferSession, TransferErrors> {
        let user_id = user.id.id;
        if let Some(ref password) = password {
            if password.len() > 20 {
                return Err(TransferErrors::PasswordLengthExceed(20))
            }
        }

        let alias = self.markov_generator.generate_name().await?;

        let session = TransferSession::public(password, user_id, alias, to_email.clone()).await;

        let session = self.transfer_repository.create(session).await?;

        Ok(session)
    }

    pub async fn update_transfer_progress(
        &self,
        user_id: u64,
        session_id: u64,
        resource_id: u64,
        transferred_amount_in_bytes: u64
    ) -> Result<(), TransferErrors> {
        let session_id = TransferSessionId {
            order_id: Some(session_id),
            user_order_id: Some(user_id)
        };

        let Some(mut session) = self.transfer_repository.find_one(&session_id).await? else {
            return Err(TransferErrors::SessionNotFound)
        };

        session.update_transferred_progress(resource_id, transferred_amount_in_bytes);

        self.transfer_repository.update_one(session).await?;

        Ok(())
    }

    pub async fn add_resources(
        &self,
        user: &User,
        session_order_id: u64,
        requests: Vec<TransferResourceRequest>
    ) -> Result<TransferResourcesResponse, TransferErrors> {
        if requests.is_empty() {
            return Err(TransferErrors::EmptyResources)
        }

        let session_id = TransferSessionId {
            order_id: Some(session_order_id),
            user_order_id: Some(user.order_id)
        };

        let Some(mut session) = self.transfer_repository.find_one(&session_id).await? else {
            return Err(TransferErrors::SessionNotFound)
        };

        for request in requests.iter() {
            session.start_transfer(request.order_id, request.name.clone(), request.size, request.r#type.clone()).await?;
        }

        let session = self.transfer_repository.update_one(session).await?;

        let Some(first_resource_id) = session.current_resource().map(|it| it.order_id()) else {
            log::warn!("The first resource must be defined, session id = {}", session.order_id());
            return Err(TransferErrors::ResourceNotFoundOrAlreadyCompleted)
        };

        let mut thumbnails = session.thumbnail_resources();

        for thumbnail in thumbnails.iter_mut() {
            let _ = self.cloud_storage.sign_upload(&mut thumbnail.1).await;
        }

        let Some(first_resource) = session.resources().iter().find(|it| it.order_id() == first_resource_id).cloned() else {
            return Err(TransferErrors::ResourceNotFoundOrAlreadyCompleted)
        };

        if let Some(ref to_email) = session.to_email() {
            let download_url = session.access_url();
            let resources = session.resources().iter().map(|it| MailFileResource {
                name: it.name().to_string(),
                size_in_bytes: it.size_in_bytes() as i32
            }).collect();

            if let Err(e) = self.email_service.send_email(&to_email, EmailTemplate {
                template: Some(Template::SendFile(SendFileTemplate {
                    sender_email: user.email.clone(),
                    sender_display_name: Some(user.display_name.clone()),
                    download_url,
                    datetime: Datetime::now(),
                    files: resources
                }))
            }).await {
                log::info!("Failed to send email to {to_email}: {e}");
            }
        }

        let response = TransferResourcesResponse {
            session_id: session_order_id,
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

        let session_order_id = session.order_id();

        let Some(current_progress) = session.current_resource_progress_mut() else {
            return Err(TransferErrors::ResourceNotFoundOrAlreadyCompleted)
        };

        let expected_id = current_progress.resource_id();
        if expected_id != resource_id {
            log::warn!("Id {resource_id} is not matched with current resource {expected_id} session_id {session_order_id}");
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
