use crate::app::operations::transfer::TransferOperationOutput;
use crate::app::operations::CoreOperationOutput;
use crate::app::repository::errors::PersistenceError;
use crate::app::repository::local_resource::LocalResourceRepository;
use crate::app::transfer::session::{TransferSession, TransferSessionStatus};
use crate::app::transfer::target::TransferTarget;
use crate::core_api::{CoreBridge, NetStream, NetStreamEvent};
use crate::rpc::cloud_server::CloudServer;
use crate::rpc::errors::RpcErrors;
use core_services::utils::maybe::MaybeSend;
use futures::channel::oneshot;
use futures_util::future::join_all;
use futures_util::lock::Mutex;
use futures_util::{join, StreamExt};
use n0_future::task::spawn;
use n0_future::time::Instant;
use schema::devlog::bitbridge::cloud_resource_message::ResourceType as ResourceTypeSchema;
use schema::devlog::bitbridge::commit_file_upload_request::UploadStatus;
use schema::devlog::bitbridge::subscribe_session_info_response::Event;
use schema::devlog::bitbridge::{ClientUploadRequest, CloudResourceMessage};
use std::collections::HashMap;
use std::sync::{Arc, Weak};

#[derive(Debug, thiserror::Error)]
pub enum CloudTransferErrors {
    #[error("{0}")]
    GrpcErrors(#[from] RpcErrors),
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
    IOError(#[from] PersistenceError)
}

pub struct CloudService<T>
where
    T: Clone,
    T: MaybeSend + Sync,
    T: tonic::client::GrpcService<tonic::body::Body>,
    T::Future: MaybeSend,
    T::Error: Into<tonic::codegen::StdError>,
    T::ResponseBody: http_body::Body<Data = bytes::Bytes> + Send + 'static,
    <T::ResponseBody as http_body::Body>::Error: Into<tonic::codegen::StdError> + Send
{
    pub server: CloudServer<T>,
    pub core_bridge: Arc<dyn CoreBridge>,
    pub active_session: Mutex<Weak<Mutex<TransferSession>>>,
    pub repository: Arc<dyn LocalResourceRepository>,
    pub net_stream: Box<dyn NetStream>
}

impl<T> CloudService<T>
where
    T: Clone,
    T: MaybeSend + Sync,
    T::Future: MaybeSend,
    T: tonic::client::GrpcService<tonic::body::Body>,
    T::Error: Into<tonic::codegen::StdError>,
    T::ResponseBody: http_body::Body<Data = bytes::Bytes> + Send + 'static,
    <T::ResponseBody as http_body::Body>::Error: Into<tonic::codegen::StdError> + Send
{
    pub async fn create_public_session(&self, mut session: TransferSession) -> Result<TransferSession, CloudTransferErrors> {
        let (password, to_emails) = match &session.target {
            TransferTarget::Internet { password, to_emails, .. } => (password.clone(), to_emails.clone()),
            _ => return Err(CloudTransferErrors::UnsupportedTransferTarget)
        };

        let user = match session.target {
            TransferTarget::Internet { from_user, .. } => from_user,
            _ => return Err(CloudTransferErrors::InvalidSessionTarget)
        };

        let response = self.server.create_public_transfer_session(password, to_emails).await?;

        session.order_id = response.order_id as u64;
        session.target = TransferTarget::Internet {
            is_required_password: response.password.is_some(),
            password: response.password,
            access_url: Some(response.access_url),
            from_user: user,
            to_emails: response.to_emails
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
                order_id: it.order_id,
                size: it.size as i64,
                thumbnail_download_url: None,
                download_url: "".to_string()
            })
            .collect();

        drop(session_guard);

        let response = self.server.add_resources(session_order_id, resources).await?;

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
        let (thumbnail_result, upload_result) = join!(
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

            let resource = match session_guard.resources.iter().find(|it| it.order_id == request.resource_order_id) {
                Some(resource) => resource,
                None => continue
            };

            let thumbnail_file_path = match &resource.thumbnail_path {
                Some(path) => path.clone(),
                None => continue
            };

            drop(session_guard);

            let Ok(size) = self.repository.size(thumbnail_file_path.clone()).await else {
                continue;
            };

            log::info!("Uploading thumbnail to {}", request.upload_url);
            let url = (request.upload_url.clone()).parse::<url::Url>().unwrap();
            let Ok(mut net_stream) = self.net_stream.upload_resource(url, thumbnail_file_path, size).await else {
                continue;
            };

            let mut rx = net_stream.start().await?;
            while let Some(event) = rx.next().await {
                if let NetStreamEvent::Error(e) = event {
                    log::warn!("Failed to upload thumbnail: {e:?}");
                    break
                }

                if let NetStreamEvent::Completed = event {
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
        let session_order_id = session.lock().await.order_id;
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
                let size = match repository.size(resource_path).await {
                    Ok(size) => size,
                    Err(e) => {
                        let _ = tx.send(Err(CloudTransferErrors::from(e)));
                        return;
                    }
                };

                let _ = tx.send(Ok(size));
            });
        }

        drop(session_guard);

        spawn(async move {
            join_all(futures).await;
        });

        while let Some(ref request) = current_upload_request {
            if session.lock().await.is_completed() {
                self.server.cancel_session(session_order_id).await?;
                return Ok(());
            }

            let order_id = request.resource_order_id;
            let upload_url = request.upload_url.clone();

            let size = match resource_size_tasks.remove(&order_id) {
                Some(rx) => match rx.await {
                    Ok(size) => size?,
                    Err(_) => continue
                },
                None => continue
            };

            current_upload_request = match self.upload_resource(session, order_id, upload_url, size, core_request_id).await {
                Ok(_) => {
                    let mut session_guard = session.lock().await;
                    let progress = session_guard.resource_mut_progress(order_id).expect("Progress not found");
                    progress.success();
                    let msg = CoreOperationOutput::Transfer(TransferOperationOutput::TransferResourceProgressUpdate(progress.clone()));
                    drop(session_guard);
                    log::info!("Resource {order_id:?} uploaded, response to {core_request_id:?}");
                    let _ = self.core_bridge.response(core_request_id, msg).await;
                    log::info!("Resource {order_id:?} uploaded, response to {core_request_id:?} done");

                    self.server.commit_file_upload(session_order_id, order_id, UploadStatus::Success, None).await?
                }
                Err(e) => {
                    log::error!("Upload resource failed with status: {e:?}");
                    let mut session_guard = session.lock().await;
                    let progress = session_guard.resource_mut_progress(order_id).expect("Progress not found");
                    progress.fail(e.to_string());
                    let msg = CoreOperationOutput::Transfer(TransferOperationOutput::TransferResourceProgressUpdate(progress.clone()));
                    drop(session_guard);
                    let _ = self.core_bridge.response(core_request_id, msg).await;

                    self.server
                        .commit_file_upload(session_order_id, order_id, UploadStatus::Failed, Some(e.to_string()))
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
        size: u64,
        core_request_id: u32
    ) -> Result<(), CloudTransferErrors> {
        let session_guard = transfer_session.lock().await;
        let resource_path = match session_guard.resources.iter().find(|it| it.order_id == resource_order_id) {
            Some(resource) => resource.path.clone(),
            None => return Err(CloudTransferErrors::ResourceNotFound)
        };

        let session_order_id = session_guard.order_id;

        drop(session_guard);

        let mut total_sent = 0;
        let url = (upload_url.clone()).parse::<url::Url>().unwrap();
        let mut net_stream = self.net_stream.upload_resource(url, resource_path.clone(), size).await?;
        let mut rx = net_stream.start().await?;
        log::info!("Uploading resource {resource_path:?} size = {size}");
        let mut ticker = Instant::now();
        let progress_update_interval = std::time::Duration::from_millis(3000);
        while let Some(event) = rx.next().await {
            let mut session_guard = transfer_session.lock().await;
            if session_guard.is_canceled() {
                net_stream.end().await?;
                return Err(CloudTransferErrors::SessionCancelled);
            }

            let progress = session_guard.resource_mut_progress(resource_order_id).expect("Progress not found");

            match event {
                NetStreamEvent::Progress { uploaded_bytes } => {
                    let count = uploaded_bytes - total_sent;
                    progress.update_progress(count);
                    total_sent = uploaded_bytes;
                    if ticker.elapsed() > progress_update_interval {
                        ticker = Instant::now();
                        self.server.update_transfer_progress(session_order_id, resource_order_id, total_sent).await?;
                    }
                }
                NetStreamEvent::Error(e) => {
                    log::warn!("Failed to upload resource: {e:?}");
                    net_stream.end().await?;
                    return Err(CloudTransferErrors::from(e));
                }
                NetStreamEvent::Completed => {
                    log::info!("Resource {resource_path:?} uploaded");
                    progress.success();
                    break;
                }
            }

            let progress_update_event =
                CoreOperationOutput::Transfer(TransferOperationOutput::TransferResourceProgressUpdate(progress.clone()));

            self.core_bridge.response_throttle(core_request_id, progress_update_event).await;
        }

        net_stream.end().await?;
        self.server.update_transfer_progress(session_order_id, resource_order_id, total_sent).await?;
        log::info!("Resource {resource_path:?} uploaded, total sent = {total_sent} bytes");

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

                if let Err(e) = self.server.cancel_session(session_id).await {
                    log::error!("Failed to cancel session: {e:?}");
                }

                return true;
            }
        }

        false
    }

    pub async fn fetch_public_session(
        &self,
        core_request_id: u32,
        session_id: u64,
        user_id: u64,
        password: Option<String>
    ) -> Result<(), CloudTransferErrors> {
        let mut stream = self.server.subscribe_public_session_events(user_id, session_id, password).await?;
        while let Some(Ok(value)) = stream.next().await {
            let Some(event) = value.event else {
                break;
            };

            let (progresses, resources) = match event {
                Event::ProgressUpdated(mut s) => (s.progress_update.drain(..).collect::<Vec<_>>(), vec![]),
                Event::SessionUpdated(s) => {
                    let mut session = s.session_updated;
                    let resources = session.resources.drain(..).collect::<Vec<_>>();
                    let progresses = session.progresses.drain(..).collect::<Vec<_>>();
                    (progresses, resources)
                }
                Event::ResourceUpdated(mut s) => (vec![], s.resource_update.drain(..).collect::<Vec<_>>())
            };

            let _ = self
                .core_bridge
                .response(
                    core_request_id,
                    CoreOperationOutput::Transfer(TransferOperationOutput::PublicTransferSessionUpdated((
                        resources.into_iter().map(|it| it.into()).collect(),
                        progresses.into_iter().map(|it| it.into()).collect()
                    )))
                )
                .await;
        }

        Ok(())
    }
}
