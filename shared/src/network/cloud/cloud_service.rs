use std::sync::{Arc, Weak};
use std::time::Duration;
use tokio::sync::OnceCell;

use core_services::local_storage::file_system::{File, Folder, IOCursor};
use schema::devlog::bitbridge::cloud_resource_message::ResourceType as ResourceTypeSchema;
use schema::devlog::bitbridge::commit_file_upload_request::UploadStatus;
use schema::devlog::bitbridge::{ClientUploadRequest, CloudResourceMessage};
use tokio::io::{duplex, AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio_util::io::ReaderStream;

use crate::app::file_system::file::ResourceType;
use crate::app::operations::transfer::TransferOperationOutput;
use crate::app::operations::CoreOperationOutput;
use crate::app::transfer::session::{TransferSession, TransferSessionStatus};
use crate::app::transfer::target::TransferTarget;
use crate::errors::NetworkError;
use crate::grpc::cloud_server::CloudServer;
use crate::native::message_to_shell::MessageToShell;
use crate::{serialize, ShellRuntime, ThrottleShellRuntime};

#[derive(Debug, thiserror::Error)]
pub enum CloudTransferErrors {
    #[error("Network error: {0}")]
    NetworkError(#[from] NetworkError),
    #[error("Invalid session target")]
    InvalidSessionTarget,
    #[error("Failed to open file: {0}")]
    FileError(String),
    #[error("Http error: {0}")]
    HttpError(#[from] reqwest::Error),
    #[error("Upload process error: {0}")]
    UploadProcessError(String),
    #[error("Only one session is allowed")]
    OnlyOneSessionAllowed,
    #[error("Session is cancelled")]
    SessionCancelled,
    #[error("Resource not found")]
    ResourceNotFound,
    #[error("Unsupported transfer target")]
    UnsupportedTransferTarget
}

pub struct CloudService {
    server: CloudServer,
    shell_runtime: OnceCell<Arc<dyn ShellRuntime>>,
    active_session: Mutex<Weak<Mutex<TransferSession>>>
}

impl CloudService {
    pub fn new(cloud: CloudServer) -> Self {
        Self {
            server: cloud,
            shell_runtime: OnceCell::new(),
            active_session: Default::default()
        }
    }

    pub fn init(&self, shell_runtime: Arc<dyn ShellRuntime>) {
        let _ = self.shell_runtime.set(shell_runtime);
    }

    pub fn shell_runtime(&self) -> &Arc<dyn ShellRuntime> {
        self.shell_runtime.get().unwrap()
    }

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

            return Ok(TransferSessionStatus::Success)
        }

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
        for request in thumbnail_upload_requests {
            let session_guard = session.lock().await;
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

            let thumbnail_file = match File::new(None, thumbnail_file_path.as_string()).await {
                Ok(file) => file,
                Err(_) => continue
            };

            let Ok(file) = thumbnail_file.open().await else {
                continue;
            };

            let Ok(file_size) = file.metadata().await else {
                continue;
            };

            let stream = ReaderStream::new(file);

            let client = reqwest::Client::new();
            let response = client
                .put(&request.upload_url)
                .header("Content-Length", file_size.len().to_string())
                .body(reqwest::Body::wrap_stream(stream))
                .send()
                .await;

            if let Err(e) = response {
                log::error!("Upload thumbnail failed with status: {}", e);
                continue;
            }
        }

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
        while let Some(request) = current_upload_request {
            if session.lock().await.is_completed() {
                self.server.cancel_session(session_order_id).await?;
                return Ok(());
            }

            let order_id = request.resource_order_id as u64;
            let upload_url = request.upload_url.clone();

            current_upload_request = match self.upload_resource(session, order_id, upload_url, core_request_id).await {
                Ok(_) => {
                    let mut session_guard = session.lock().await;
                    let progress = session_guard.resource_mut_progress(order_id).expect("Progress not found");
                    progress.success();
                    let msg = CoreOperationOutput::Transfer(TransferOperationOutput::TransferResourceProgressUpdate(progress.clone()));
                    drop(session_guard);
                    self.shell_runtime()
                        .msg_from_native(serialize(&MessageToShell::HandleResponse(core_request_id, msg)))
                        .await;

                    self.server
                        .commit_file_upload(session_order_id, order_id as i64, UploadStatus::Success, None)
                        .await?
                }
                Err(e) => {
                    log::error!("Upload resource failed with status: {:?}", e);
                    let mut session_guard = session.lock().await;
                    let progress = session_guard.resource_mut_progress(order_id).expect("Progress not found");
                    progress.fail(e.to_string());
                    let msg = CoreOperationOutput::Transfer(TransferOperationOutput::TransferResourceProgressUpdate(progress.clone()));
                    drop(session_guard);
                    self.shell_runtime()
                        .msg_from_native(serialize(&MessageToShell::HandleResponse(core_request_id, msg)))
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
        core_request_id: u32
    ) -> Result<(), CloudTransferErrors> {
        let upload_chunk_size = 1024 * 1024;
        let max_buffer_size = upload_chunk_size * 5;
        let resource_type = match transfer_session.lock().await.resources.iter().find(|it| it.order_id == resource_order_id) {
            Some(resource) => resource.r#type.clone(),
            None => return Err(CloudTransferErrors::ResourceNotFound)
        };

        let resource_path = match transfer_session.lock().await.resources.iter().find(|it| it.order_id == resource_order_id) {
            Some(resource) => resource.path.as_string(),
            None => return Err(CloudTransferErrors::ResourceNotFound)
        };

        let mut cursor = match resource_type {
            ResourceType::Folder => {
                let folder = Folder::new(resource_path).await.map_err(|it| CloudTransferErrors::FileError(it.to_string()))?;
                folder
                    .cursor(upload_chunk_size)
                    .await
                    .map_err(|it| CloudTransferErrors::FileError(it.to_string()))?
            }
            _ => {
                let file = File::new(None, resource_path).await.map_err(|it| CloudTransferErrors::FileError(it.to_string()))?;
                file.cursor(0, upload_chunk_size)
                    .await
                    .map_err(|it| CloudTransferErrors::FileError(it.to_string()))?
            }
        };

        let (writer, reader) = duplex(max_buffer_size);
        let writer = Arc::new(Mutex::new(writer));
        let stream = ReaderStream::new(reader);
        let body = reqwest::Body::wrap_stream(stream);

        let handle: JoinHandle<Result<(), CloudTransferErrors>> = tokio::spawn(async move {
            let client = reqwest::Client::new();
            let response = client.put(&upload_url).header("Content-Type", "application/octet-stream").body(body).send().await?;

            Ok(())
        });

        let mut upload_handle: Option<JoinHandle<Result<(), CloudTransferErrors>>> = None;
        let progress_sender = ThrottleShellRuntime::new(self.shell_runtime().clone(), Duration::from_millis(800));
        while let Some(chunk) = cursor.next().await.map_err(|it| CloudTransferErrors::FileError(it.to_string()))? {
            let count = chunk.len();
            let writer = writer.clone();

            let mut session_guard = transfer_session.lock().await;
            if session_guard.is_canceled() {
                return Err(CloudTransferErrors::SessionCancelled);
            }

            let progress = session_guard.resource_mut_progress(resource_order_id).expect("Progress not found");
            progress.update_progress(count as u64);
            let progress_update_event =
                CoreOperationOutput::Transfer(TransferOperationOutput::TransferResourceProgressUpdate(progress.clone()));
            drop(session_guard);

            progress_sender.send(MessageToShell::HandleResponse(core_request_id, progress_update_event)).await;

            if let Some(handle) = upload_handle.take() {
                handle.await.map_err(|it| CloudTransferErrors::FileError(it.to_string()))??;
            }

            upload_handle = Some(tokio::spawn(async move {
                writer
                    .lock()
                    .await
                    .write_all(&chunk)
                    .await
                    .map_err(|it| CloudTransferErrors::UploadProcessError(it.to_string()))?;

                Ok(())
            }));
        }

        if let Some(handle) = upload_handle.take() {
            handle.await.map_err(|it| CloudTransferErrors::FileError(it.to_string()))??;
        }

        let session_guard = transfer_session.lock().await;
        let progress = session_guard.resource_progress(resource_order_id).expect("Progress not found");
        if !progress.is_completed() {
            return Err(CloudTransferErrors::UploadProcessError(
                "Upload process is interrupted".to_string()
            ));
        }

        if let Ok(handle) = handle.await {
            handle?;
        }

        Ok(())
    }
}
