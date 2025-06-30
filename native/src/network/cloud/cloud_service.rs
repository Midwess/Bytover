use futures_util::future::join_all;
use std::collections::HashMap;
use std::sync::{Arc, Weak};
use std::time::Duration;
use log::info;
use tokio::sync::{oneshot, OnceCell};

use core_services::local_storage::file_system::{File, Folder};
use schema::devlog::bitbridge::cloud_resource_message::ResourceType as ResourceTypeSchema;
use schema::devlog::bitbridge::commit_file_upload_request::UploadStatus;
use schema::devlog::bitbridge::{ClientUploadRequest, CloudResourceMessage};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use crate::grpc::cloud_server::CloudServer;
use crate::grpc::errors::NativeGrpcErrors;
use crate::native::message_to_shell::MessageToShell;
use shared::app::file_system::file::ResourceType;
use shared::app::operations::transfer::TransferOperationOutput;
use shared::app::operations::CoreOperationOutput;
use shared::app::repository::errors::PersistenceError;
use shared::app::repository::local_resource::LocalResourceRepository;
use shared::app::transfer::session::{TransferSession, TransferSessionStatus};
use shared::app::transfer::target::TransferTarget;
use shared::core_api::{CoreBridge, IOReader, NetStream};

#[derive(Debug, thiserror::Error)]
pub enum CloudTransferErrors {
    #[error("Network error: {0}")]
    GrpcErrors(#[from] NativeGrpcErrors),
    #[error("Invalid session target")]
    InvalidSessionTarget,
    #[error("Failed to open file: {0}")]
    FileError(String),
    #[error("Upload process error: {0}")]
    UploadProcessError(String),
    #[error("Only one session is allowed")]
    OnlyOneSessionAllowed,
    #[error("Session is cancelled")]
    SessionCancelled,
    #[error("Resource not found")]
    ResourceNotFound,
    #[error("Resource error")]
    ResourceError(String),
    #[error("Unsupported transfer target")]
    UnsupportedTransferTarget,
    #[error("Internal error {0}")]
    InternalError(#[from] anyhow::Error),
    #[error("IO Error {0}")]
    IOError(#[from] PersistenceError),
}

pub struct CloudService {
    pub server: CloudServer,
    pub core_bridge: Arc<dyn CoreBridge>,
    pub active_session: Mutex<Weak<Mutex<TransferSession>>>,
    pub repository: Arc<dyn LocalResourceRepository>,
    pub net_stream: Box<dyn NetStream>
}

impl CloudService {
    pub async fn create_public_session(&self, mut session: TransferSession) -> Result<TransferSession, CloudTransferErrors> {
        let password = match &session.target {
            TransferTarget::Internet { password, .. } => password.clone(),
            _ => return Err(CloudTransferErrors::UnsupportedTransferTarget)
        };

        let response = self.server.create_public_transfer_session(password).await?;

        session.order_id = response.order_id as u64;
        session.target = TransferTarget::Internet {
            password: response.password,
            access_url: Some(response.access_url)
        };

        Ok(session)
    }

    pub async fn send_session(
        &self,
        session: TransferSession,
        core_request_id: u32
    ) -> Result<TransferSessionStatus, CloudTransferErrors> {
        let mut session_guard = self.active_session.lock().await;
        if session_guard.upgrade().is_some() {
            return Err(CloudTransferErrors::OnlyOneSessionAllowed);
        }

        let session = Arc::new(Mutex::new(session));
        *session_guard = Arc::downgrade(&session);

        drop(session_guard);

        let session_guard = session.lock().await;
        let session_order_id = session_guard.order_id;
        let resources = session_guard
            .resources
            .iter()
            .map(|it| CloudResourceMessage {
                r#type: ResourceTypeSchema::from(&it.r#type).into(),
                name: it.name.clone(),
                order_id: it.order_id as i64,
                size: it.size as i64
            })
            .collect();

        drop(session_guard);

        let response = self.server.add_resources(session_order_id as i64, resources).await?;

        let session_guard = session.lock().await;
        if session_guard.is_completed() {
            if session_guard.is_canceled() {
                return Ok(TransferSessionStatus::Canceled)
            };

            return Ok(TransferSessionStatus::Success);
        }

        drop(session_guard);

        log::info!("Start uploading resources and thumbnails");

        let thumbnail_upload_requests = response.thumbnail_upload_requests;
        let (thumbnail_result, upload_result) = tokio::join!(
            self.upload_thumbnails(&session, thumbnail_upload_requests),
            self.upload_resources(&session, response.first_resource_upload_request, core_request_id)
        );

        thumbnail_result?;
        upload_result?;

        Ok(TransferSessionStatus::Success)
    }

    pub async fn upload_thumbnails(
        &self,
        session: &Arc<Mutex<TransferSession>>,
        thumbnail_upload_requests: Vec<ClientUploadRequest>
    ) -> Result<(), CloudTransferErrors> {
        log::info!("Uploading thumbnails");
        for request in thumbnail_upload_requests {
            let session_guard = session.lock().await;
            log::info!("Uploading thumbnail {}", request.resource_order_id);
            if session_guard.is_canceled() {
                return Ok(());
            }

            let resource = match session_guard.resources.iter().find(|it| it.order_id == request.resource_order_id as u64) {
                Some(resource) => resource,
                None => continue
            };

            let thumbnail_file_path = match &resource.thumbnail_path {
                Some(path) => path.clone(),
                None => continue
            };

            drop(session_guard);

            let Ok(mut cursor) = self.repository.read(thumbnail_file_path, 1024 * 1024).await else {
                continue;
            };

            let Ok(size) = cursor.total_size().await else {
                continue;
            };

            log::info!("Uploading thumbnail to {}", request.upload_url);
            let url = (request.upload_url.clone()).parse::<url::Url>().unwrap();
            let Ok(mut net_stream) = self.net_stream.start(url, size).await else {
                continue;
            };

            while let Ok(Some(bytes)) = cursor.next().await {
                if let Err(e) = net_stream.write(bytes).await {
                    log::warn!("Failed to upload thumbnail: {e:?}");
                    break
                }
            }

            let _ = net_stream.end().await;
        }

        log::info!("Thumbnails uploaded");

        Ok(())
    }

    pub async fn upload_resources(
        &self,
        session: &Arc<Mutex<TransferSession>>,
        first_upload_request: ClientUploadRequest,
        core_request_id: u32
    ) -> Result<(), CloudTransferErrors> {
        let session_order_id = session.lock().await.order_id as i64;
        let mut current_upload_request = Some(first_upload_request);

        let mut resource_size_tasks = HashMap::new();
        let mut futures = Vec::new();

        let session_guard = session.lock().await;

        for resource in session_guard.resources.iter() {
            let (tx, rx) = oneshot::channel();
            resource_size_tasks.insert(resource.order_id, rx);

            let resource_path = resource.path.clone();
            let repository = self.repository.clone();
            futures.push(async move {
                let cursor = match repository.read(resource_path, 1024 * 1024).await {
                    Ok(cursor) => cursor,
                    Err(e) => {
                        let _ = tx.send(Err(CloudTransferErrors::from(e)));
                        return;
                    }
                };

                if let Err(e) = cursor.total_size().await {
                    let _ = tx.send(Err(CloudTransferErrors::from(e)));
                    return;
                };

                let _ = tx.send(Ok(cursor));
            });
        }

        drop(session_guard);

        tokio::spawn(async move {
            join_all(futures).await;
        });

        while let Some(ref request) = current_upload_request {
            if session.lock().await.is_completed() {
                self.server.cancel_session(session_order_id).await?;
                return Ok(());
            }

            let order_id = request.resource_order_id as u64;
            let upload_url = request.upload_url.clone();

            let cursor = match resource_size_tasks.remove(&order_id) {
                Some(rx) => match rx.await {
                    Ok(size) => size?,
                    Err(_) => continue
                },
                None => continue
            };

            current_upload_request = match self.upload_resource(session, order_id, upload_url, cursor, core_request_id).await {
                Ok(_) => {
                    let mut session_guard = session.lock().await;
                    let progress = session_guard.resource_mut_progress(order_id).expect("Progress not found");
                    progress.success();
                    let msg = CoreOperationOutput::Transfer(TransferOperationOutput::TransferResourceProgressUpdate(progress.clone()));
                    drop(session_guard);
                    let _ = self.core_bridge
                        .response(core_request_id, msg)
                        .await;

                    self.server
                        .commit_file_upload(session_order_id, order_id as i64, UploadStatus::Success, None)
                        .await?
                }
                Err(e) => {
                    log::error!("Upload resource failed with status: {e:?}");
                    let mut session_guard = session.lock().await;
                    let progress = session_guard.resource_mut_progress(order_id).expect("Progress not found");
                    progress.fail(e.to_string());
                    let msg = CoreOperationOutput::Transfer(TransferOperationOutput::TransferResourceProgressUpdate(progress.clone()));
                    drop(session_guard);
                    let _ = self.core_bridge
                        .response(core_request_id, msg)
                        .await;

                    self.server
                        .commit_file_upload(session_order_id, order_id as i64, UploadStatus::Failed, Some(e.to_string()))
                        .await?
                }
            }
        }

        Ok(())
    }

    async fn upload_resource(
        &self,
        transfer_session: &Arc<Mutex<TransferSession>>,
        resource_order_id: u64,
        upload_url: String,
        mut cursor: Box<dyn IOReader>,
        core_request_id: u32
    ) -> Result<(), CloudTransferErrors> {
        let resource_path = match transfer_session.lock().await.resources.iter().find(|it| it.order_id == resource_order_id) {
            Some(resource) => resource.path.as_string(),
            None => return Err(CloudTransferErrors::ResourceNotFound)
        };

        let total_size = cursor.total_size().await?;
        log::info!("Uploading resource {resource_path} size = {total_size}");
        let mut total_sent = 0;
        let url = (upload_url.clone()).parse::<url::Url>().unwrap();
        let mut net_stream = self.net_stream.start(url, total_size).await?;
        while let Some(chunk) = cursor.next().await.map_err(|it| CloudTransferErrors::FileError(it.to_string()))? {
            total_sent += chunk.len();
            let count = chunk.len();

            let mut session_guard = transfer_session.lock().await;
            if session_guard.is_canceled() {
                return Err(CloudTransferErrors::SessionCancelled);
            }

            let progress = session_guard.resource_mut_progress(resource_order_id).expect("Progress not found");
            progress.update_progress(count as u64);
            let progress_update_event =
                CoreOperationOutput::Transfer(TransferOperationOutput::TransferResourceProgressUpdate(progress.clone()));
            drop(session_guard);

            self.core_bridge.response_throttle(core_request_id, progress_update_event).await;

            net_stream
                .write(chunk)
                .await
                .map_err(|it| CloudTransferErrors::UploadProcessError(it.to_string()))?;
        }

        net_stream.end().await?;

        let session_guard = transfer_session.lock().await;
        let progress = session_guard.resource_progress(resource_order_id).expect("Progress not found");
        if !progress.is_completed() {
            return Err(CloudTransferErrors::UploadProcessError(
                "Upload process is interrupted".to_string()
            ));
        }

        Ok(())
    }

    pub async fn cancel(&self, session_id: u64) -> bool {
        let session_guard = self.active_session.lock().await.clone();
        if let Some(session) = session_guard.upgrade() {
            let mut session = session.lock().await;
            if session.order_id == session_id {
                log::info!(target: "cloud", "Cancelling cloud session: {session_id:?}");
                session.cancel();
                drop(session);

                if let Err(e) = self.server.cancel_session(session_id as i64).await {
                    log::error!("Failed to cancel session: {e:?}");
                }

                return true;
            }
        }

        false
    }
}
