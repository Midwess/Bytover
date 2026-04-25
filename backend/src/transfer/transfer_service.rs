use crate::app_gateway::app_info::{AppInfoErrors, AppInfoService};
use crate::app_gateway::markov::{Markov, MarkovErrors};
use crate::cloud_storage::storage::{CloudStorage, CloudStorageErrors};
use crate::entities::transfer_progress::{TransferProgressErrors, TransferProgressStatus};
use crate::entities::transfer_resource::{TransferResource, TransferResourceType};
use crate::entities::transfer_session::{TransferSession, TransferSessionErrors};
use crate::entities::user_capabilities::UserCapabilities;
use crate::mail::service::EmailService;
use crate::repositories::transfer_session::{TransferSessionId, TransferSessionRepository};
use crate::repositories::user_capabilities::UserCapabilitiesRepository;
use core_services::db::repository::abstraction::errors::RepositoryError;
use futures::join;
use schema::crafter::email_template::Template::{self};
use schema::crafter::{EmailTemplate, FileResource as MailFileResource, SendFileTemplate};
use schema::devlog::app_gateway::models::{Application, Device, User};
use schema::devlog::bitbridge::client_upload_request::Upload;
use schema::devlog::bitbridge::update_transfer_progress_request::Status as ClientUploadStatus;
use schema::value::datetime::Datetime;
use schema::value::static_resource::StaticResource;
use tokio::time::Instant;

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
    MarkovError(#[from] MarkovErrors),
    #[error("Application service error {0}")]
    ApplicationServiceError(#[from] AppInfoErrors),
    #[error("Current plan does not allow password-protected sessions")]
    PlanForbidsPasswordEncryption,
    #[error("File count {total} exceeds the per-transfer cap {cap}")]
    FileCountExceed { total: u32, cap: u32 },
    #[error("Lifetime transfer cap reached: cap {cap}, used {used}, requested {requested}")]
    LifetimeBytesExceed { cap: u64, used: u64, requested: u64 },
}

pub struct TransferResourceRequest {
    // The user can decide the order id
    // it cannot be duplicated
    pub order_id: Option<u64>,
    pub name: String,
    pub r#type: TransferResourceType,
    pub size: u64,
}

pub struct TransferResourcesResponse {
    pub session_id: u64,
    pub first_resource: TransferResource,
    pub first_resource_upload_request: Upload,
    pub thumbnail_upload_urls: Vec<(u64, String)>,
    pub thumbnails: Vec<(u64, StaticResource)>,
}

pub struct TransferService {
    pub transfer_repository: Box<dyn TransferSessionRepository>,
    pub cloud_storage: Box<dyn CloudStorage>,
    pub app_service: Box<dyn AppInfoService>,
    pub markov_generator: Box<dyn Markov>,
    pub email_service: Box<dyn EmailService>,
    pub user_capabilities_repository: std::sync::Arc<dyn UserCapabilitiesRepository>,
}

impl TransferService {
    pub async fn create_public_transfer_session(
        &self,
        user: &User,
        capabilities: &UserCapabilities,
        password: Option<String>,
        to_emails: Vec<String>,
    ) -> Result<TransferSession, TransferErrors> {
        let user_id = user.id.id;
        let mut password = password.map(|it| it.trim().to_owned());
        if let Some(ref value) = password {
            if !value.is_empty() && !capabilities.password_encryption_allowed() {
                return Err(TransferErrors::PlanForbidsPasswordEncryption);
            }

            if value.len() > 20 {
                return Err(TransferErrors::PasswordLengthExceed(20));
            }

            if value.is_empty() {
                password.take();
            }
        }

        let alias = self.markov_generator.generate_name().await?;

        let session = TransferSession::public(password, user_id, alias, to_emails).await;

        let session = self.transfer_repository.create(session).await?;

        Ok(session)
    }

    pub async fn update_transfer_progress(
        &self,
        user: &User,
        device: &Device,
        session_id: u64,
        resource_id: u64,
        status: &ClientUploadStatus,
    ) -> Result<Option<(u64, Upload)>, TransferErrors> {
        let session_id = TransferSessionId {
            order_id: Some(session_id),
            user_order_id: Some(user.order_id),
        };

        let Some(mut session) = self.transfer_repository.find_one(&session_id).await? else {
            return Err(TransferErrors::SessionNotFound);
        };

        match status {
            ClientUploadStatus::TransferredAmountInBytes(transferred_amount) => {
                session.update_transferred_progress(resource_id, *transferred_amount as u64);
                self.transfer_repository.update_one(session).await?;
                Ok(None)
            }
            ClientUploadStatus::Success(completion) => {
                let next_resource = session.next_resource();
                let next_resource_id = next_resource.map(|it| it.order_id());
                let platform = device.platform();
                let gen_upload_request = async {
                    if let Some(next_resource) = next_resource {
                        return self.cloud_storage.get_upload_solution(user, platform, next_resource).await.map(Some);
                    }

                    Ok(None)
                };

                let (next_upload_result, complete_upload_result) =
                    join!(gen_upload_request, self.cloud_storage.complete_upload(user, completion));
                let current_resource_id = session.current_resource().map(|it| it.order_id()).unwrap_or(0);

                if let Err(e) = complete_upload_result {
                    let Some(p) = session.current_resource_progress_mut() else {
                        return Err(TransferErrors::ResourceNotFoundOrAlreadyCompleted);
                    };
                    p.cancel();
                    self.transfer_repository.update_one(session).await?;
                    log::info!("Failed to complete upload for completion {completion:?}: {e}");
                    return Err(TransferErrors::CloudStorageError(e));
                }

                if current_resource_id != resource_id {
                    log::warn!(
                        "Id {resource_id} is not matched with current resource {current_resource_id} session_id {}",
                        session.order_id()
                    );

                    return Err(TransferErrors::ResourceNotFoundOrAlreadyCompleted);
                }

                let completed_size = session
                    .resources()
                    .iter()
                    .find(|r| r.order_id() == resource_id)
                    .map(|r| r.size_in_bytes())
                    .unwrap_or(0);

                if completed_size > 0 {
                    let outcome = self
                        .user_capabilities_repository
                        .increment_bytes_used(user.order_id, completed_size)
                        .await?;
                    if outcome.cap_crossed {
                        log::info!(
                            "[transfer] user crossed lifetime cap on cloud upload: user_order_id={} new_used={} lifetime_cap={}",
                            user.order_id, outcome.new_bytes_used, outcome.lifetime_cap
                        );
                    }
                }

                let Some(current_progress) = session.current_resource_progress_mut() else {
                    return Err(TransferErrors::ResourceNotFoundOrAlreadyCompleted);
                };
                current_progress.commit(TransferProgressStatus::Success)?;

                let mut session = self.transfer_repository.update_one(session).await?;

                match next_upload_result {
                    Err(e) => {
                        if let Some(next_progress) = session.current_resource_progress_mut() {
                            next_progress.cancel();
                            self.transfer_repository.update_one(session).await?;
                        }
                        log::info!("Failed to generate upload solution for next resource: {e}");
                        Err(TransferErrors::CloudStorageError(e))
                    }
                    Ok(None) => Ok(None),
                    Ok(Some(next_upload_request)) => Ok(Some((next_resource_id.unwrap_or(0), next_upload_request))),
                }
            }
            ClientUploadStatus::Failed(error_message) => {
                let Some(current_progress) = session.current_resource_progress_mut() else {
                    return Err(TransferErrors::ResourceNotFoundOrAlreadyCompleted);
                };

                current_progress.commit(TransferProgressStatus::Failed(error_message.clone()))?;
                self.transfer_repository.update_one(session).await?;
                Ok(None)
            }
        }
    }

    pub async fn add_resources(
        &self,
        user: &User,
        device: &Device,
        app: &Application,
        capabilities: &UserCapabilities,
        session_order_id: u64,
        requests: Vec<TransferResourceRequest>,
    ) -> Result<TransferResourcesResponse, TransferErrors> {
        if requests.is_empty() {
            return Err(TransferErrors::EmptyResources);
        }

        let session_id = TransferSessionId {
            order_id: Some(session_order_id),
            user_order_id: Some(user.order_id),
        };

        let Some(mut session) = self.transfer_repository.find_one(&session_id).await? else {
            return Err(TransferErrors::SessionNotFound);
        };

        let existing_count = session.resources().len() as u32;
        let incoming_count = requests.len() as u32;
        let total_count = existing_count.saturating_add(incoming_count);
        if let Some(cap) = capabilities.would_exceed_file_count(total_count) {
            return Err(TransferErrors::FileCountExceed { total: total_count, cap });
        }

        let incoming_bytes: u64 = requests.iter().map(|r| r.size).sum();
        if capabilities.would_exceed_lifetime_cap(incoming_bytes) {
            return Err(TransferErrors::LifetimeBytesExceed {
                cap: capabilities.total_transfer_bytes_lifetime_cap(),
                used: capabilities.total_transfer_bytes_used(),
                requested: incoming_bytes,
            });
        }

        for request in requests.iter() {
            session
                .start_transfer(request.order_id, request.name.clone(), request.size, request.r#type.clone())
                .await?;
        }

        let session = self.transfer_repository.update_one(session).await?;

        let Some(first_resource_id) = session.current_resource().map(|it| it.order_id()) else {
            log::warn!("The first resource must be defined, session id = {}", session.order_id());
            return Err(TransferErrors::ResourceNotFoundOrAlreadyCompleted);
        };

        let mut thumbnails = session.thumbnail_resources();

        for thumbnail in thumbnails.iter_mut() {
            let _ = Upload::SingleUrl(self.cloud_storage.get_upload_url(&thumbnail.1).await?);
        }

        let Some(first_resource) = session.resources().iter().find(|it| it.order_id() == first_resource_id).cloned() else {
            return Err(TransferErrors::ResourceNotFoundOrAlreadyCompleted);
        };

        let platform = device.platform();

        let download_url = session.access_url(app.web_url().to_owned());
        let resources = session
            .resources()
            .iter()
            .map(|it| MailFileResource {
                name: it.name().to_string(),
                size_in_bytes: it.size_in_bytes() as i32,
            })
            .collect::<Vec<_>>();

        let template = EmailTemplate {
            template: Some(Template::SendFile(SendFileTemplate {
                sender_email: user.email.clone(),
                sender_display_name: Some(user.display_name.clone()),
                download_url,
                datetime: Datetime::now(),
                files: resources,
            })),
        };

        for to_email in session.to_emails() {
            if let Err(e) = self.email_service.send_email(to_email, template.clone()).await {
                log::info!("Failed to send email to {to_email}: {e}");
            }
        }

        let instant = Instant::now();
        let first_resource_future = self.cloud_storage.get_upload_solution(user, platform, &first_resource);

        let thumbnail_futures = thumbnails.iter().map(|(order_id, source)| {
            let cloud = &self.cloud_storage;

            async move { (order_id, cloud.get_upload_url(source).await) }
        });

        let (first_resource_upload_request, thumbnail_upload_urls) = tokio::join!(first_resource_future, async {
            let results = futures::future::join_all(thumbnail_futures).await;
            let mut collected = Vec::with_capacity(results.len());
            for (order_id, result) in results {
                collected.push((*order_id, result?));
            }

            Ok::<_, CloudStorageErrors>(collected)
        });

        let first_resource_upload_request = first_resource_upload_request?;
        let thumbnail_upload_urls = thumbnail_upload_urls?;
        log::info!(
            "Upload solution for first resource is ready in {} ms",
            instant.elapsed().as_millis()
        );
        let response = TransferResourcesResponse {
            session_id: session_order_id,
            first_resource,
            thumbnails,
            first_resource_upload_request,
            thumbnail_upload_urls,
        };

        Ok(response)
    }

    pub async fn cancel_transfer(&self, user_id: u64, session_order_id: u64) -> Result<(), TransferErrors> {
        let session_id = TransferSessionId {
            order_id: Some(session_order_id),
            user_order_id: Some(user_id),
        };

        let Some(mut session) = self.transfer_repository.find_one(&session_id).await? else {
            return Err(TransferErrors::SessionNotFound);
        };

        session.cancel();

        let _ = self.transfer_repository.update_one(session).await?;

        Ok(())
    }
}
