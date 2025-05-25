use crate::app::file_system::file::ResourceType;
use crate::app::operations::transfer::TransferOperationOutput;
use crate::app::operations::CoreOperationOutput;
use crate::app::transfer::session::{TransferSession, TransferSessionStatus, TransferStatus};
use crate::native::message_to_shell::MessageToShell;
use crate::{ShellRuntime, ThrottleShellRuntime};
use core_services::local_storage::file_system::{File, Folder};
use futures_util::{SinkExt, Stream};
use std::ops::Deref;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Weak};
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::spawn;
use tokio::sync::{oneshot, Mutex};
use tokio::task::JoinHandle;
use tokio::time::timeout;
use webrtc::data_channel::RTCDataChannel;

use super::connection::ConnectionWebRtc;
use super::throughput::ThroughputController;
use std::pin::Pin;
use std::task::{Context, Poll};
use thiserror::Error;
use tokio::sync::mpsc;

#[derive(Error, Debug)]
pub enum DataChannelError {
    #[error("Invalid format of data channel label")]
    InvalidLabelFormat(String),
    #[error("Data channel error {:?}", .0)]
    WebRtcError(#[from] webrtc::Error),
    #[error("Failed to open data channel")]
    OpenDataChannelError(String),
    #[error("File not exists")]
    FileError(String),
    #[error("Resource not found")]
    ResourceNotFound,
    #[error("Session canceled")]
    SessionCanceled,
    #[error("Data channel is closed")]
    DataChannelClosed(String),
    #[error("Data corrupted")]
    DataCorrupted,
    #[error("Timeout")]
    Timeout(Duration),
    #[error("The throughput controller error")]
    ThroughputController(String)
}

pub struct DataChannel {
    data_channel: Arc<RTCDataChannel>,
    shell_runtime: Arc<dyn ShellRuntime>,
    throughput_controller: Arc<ThroughputController>,
    session: Weak<Mutex<TransferSession>>,
    pub resource_id: u64,
    pub session_id: u64,
    pub auto_close: AtomicBool
}

impl DataChannel {
    pub fn data_label(resource_id: u64, session_id: u64) -> String {
        format!("{resource_id}-{session_id}")
    }

    pub fn from_label(label: &str) -> Result<(u64, u64), DataChannelError> {
        let parts = label.split('-').collect::<Vec<&str>>();
        if parts.len() != 2 {
            return Err(DataChannelError::InvalidLabelFormat(label.to_string()));
        }

        let resource_id = parts[0].parse::<u64>().map_err(|_| DataChannelError::InvalidLabelFormat(label.to_string()))?;
        let session_id = parts[1].parse::<u64>().map_err(|_| DataChannelError::InvalidLabelFormat(label.to_string()))?;
        Ok((resource_id, session_id))
    }

    pub async fn from_channel(
        data_channel: Arc<RTCDataChannel>,
        shell_runtime: Arc<dyn ShellRuntime>,
        throughput_controller: Arc<ThroughputController>,
        session: Weak<Mutex<TransferSession>>
    ) -> Result<Self, DataChannelError> {
        let label = data_channel.label().to_owned();
        let (resource_id, session_id) =
            DataChannel::from_label(&label).map_err(|e| DataChannelError::InvalidLabelFormat(label.clone()))?;

        Ok(Self {
            data_channel,
            shell_runtime,
            throughput_controller,
            session,
            resource_id,
            session_id,
            auto_close: AtomicBool::new(false)
        })
    }

    pub async fn stream_resource(
        connection: &ConnectionWebRtc,
        shell_runtime: Arc<dyn ShellRuntime>,
        throughput_controller: Arc<ThroughputController>,
        session_ref: Weak<Mutex<TransferSession>>,
        resource_id: u64
    ) -> Result<Self, DataChannelError> {
        let Some(session) = session_ref.clone().upgrade() else {
            return Err(DataChannelError::SessionCanceled);
        };

        let session_id = session.lock().await.order_id;
        let label = DataChannel::data_label(resource_id, session_id);
        let data_channel = connection
            .peer_connection
            .create_data_channel(label.as_str(), Some(ConnectionWebRtc::channel_config()))
            .await?;

        let (open_sender, open_receiver) = oneshot::channel();
        data_channel.on_open(Box::new(move || {
            let _ = open_sender.send(());
            Box::pin(async move {})
        }));

        let Ok(_) = timeout(Duration::from_secs(10), open_receiver).await else {
            return Err(DataChannelError::OpenDataChannelError(format!(
                "Data channel {label} is not open"
            )));
        };

        log::info!(target: "nearby", "Data channel created: {}", label);
        Ok(Self {
            data_channel,
            shell_runtime,
            throughput_controller,
            session: session_ref,
            resource_id,
            session_id,
            auto_close: AtomicBool::new(false)
        })
    }

    pub fn auto_close(&self, value: bool) {
        self.auto_close.store(value, Ordering::Relaxed);
    }

    pub async fn stop_transfer(&self) {
        let _ = self.data_channel.close().await;
    }

    pub async fn is_canceled(&self) -> bool {
        let Some(session) = self.session.upgrade() else {
            return true;
        };

        let session = session.lock().await;
        session.status() == TransferSessionStatus::Canceled
    }

    pub async fn start_download(&self, core_request_id: u32) -> Result<(), DataChannelError> {
        let mut stream = RTCStreamChannel::new(self.data_channel.clone());

        let Some(session) = self.session.upgrade() else {
            return Err(DataChannelError::SessionCanceled);
        };

        let session_guard = session.lock().await;
        let resource = session_guard
            .resources
            .iter()
            .find(|it| it.order_id == self.resource_id)
            .expect("Resource not found");

        let Some(saved_path) = resource.path.disk_path() else {
            return Err(DataChannelError::FileError("Only support absolute path".to_string()))
        };

        let file = File::new(None, saved_path.clone()).await.map_err(|e| DataChannelError::FileError(e.to_string()))?;
        let mut file = file.open().await.map_err(|e| DataChannelError::FileError(e.to_string()))?;

        log::info!(target: "nearby", "Start downloading file into: {}, size = {}", saved_path, resource.size);
        drop(session_guard);

        let progress_sender = ThrottleShellRuntime::new(self.shell_runtime.clone(), Duration::from_millis(800));

        let result = loop {
            let Some(session) = self.session.upgrade() else {
                return Err(DataChannelError::SessionCanceled);
            };

            let next_bytes = match self.throughput_controller.next_bytes(&mut stream).await {
                Ok(Some(bytes)) => bytes,
                Ok(None) => break Err(DataChannelError::DataCorrupted),
                Err(e) => break Err(e)
            };

            let written_bytes = file.write(&next_bytes).await.map_err(|e| DataChannelError::FileError(e.to_string()))?;

            if written_bytes < next_bytes.len() {
                break Err(DataChannelError::DataCorrupted);
            }

            let mut session_guard = session.lock().await;
            let progress = session_guard.resource_mut_progress(self.resource_id).expect("Progress not found");
            progress.update_progress(written_bytes as u64);
            let _ = progress_sender
                .send(MessageToShell::HandleResponse(
                    core_request_id,
                    CoreOperationOutput::Transfer(TransferOperationOutput::TransferResourceProgressUpdate(progress.clone()))
                ))
                .await;

            if progress.status.is_completed() {
                log::info!(target: "nearby", "Resource {:?} is already completed with status {:?}", self.resource_id, progress.status);
                break Ok(());
            }
        };

        let Some(session) = self.session.upgrade() else {
            return Err(DataChannelError::SessionCanceled);
        };

        let mut session_guard = session.lock().await;
        let progress = session_guard.resource_mut_progress(self.resource_id).expect("Progress not found");

        if let Err(e) = result {
            return Err(e);
        } else if !progress.is_completed() {
            return Err(DataChannelError::DataCorrupted);
        }

        Ok(())
    }

    pub async fn start_upload(&self, core_request_id: u32) -> Result<(), DataChannelError> {
        let Some(session) = self.session.upgrade() else {
            return Err(DataChannelError::SessionCanceled);
        };

        let session_guard = session.lock().await;
        let resource = session_guard
            .resources
            .iter()
            .find(|it| it.order_id == self.resource_id)
            .expect("Resource not found");

        let Some(saved_path) = resource.path.disk_path() else {
            return Err(DataChannelError::FileError("Only support absolute path".to_string()))
        };

        let progress_sender = ThrottleShellRuntime::new(self.shell_runtime.clone(), Duration::from_millis(800));

        let unreliable_size = resource.size;
        // The larger the buffer size, the more cpu efficient the upload
        // But it will cause the memory usage increase
        let mut cursor = if resource.r#type == ResourceType::Folder {
            let folder = Folder::new(saved_path.clone()).await.map_err(|e| DataChannelError::FileError(e.to_string()))?;
            folder.cursor(1024 * 1024).await.map_err(|e| DataChannelError::FileError(e.to_string()))?
        } else {
            let file = File::existing(saved_path.clone()).await.map_err(|e| DataChannelError::FileError(e.to_string()))?;
            file.cursor(0, 1024 * 1024).await.map_err(|e| DataChannelError::FileError(e.to_string()))?
        };

        drop(session_guard);

        log::info!(target: "nearby", "Start uploading file: {saved_path} size = {unreliable_size}");
        let mut last_sent_handle: Option<JoinHandle<Result<usize, DataChannelError>>> = None;
        while let Some(bytes) = cursor.next().await.map_err(|e| DataChannelError::FileError(format!("{e:?}")))? {
            let Some(session) = self.session.upgrade() else {
                return Err(DataChannelError::SessionCanceled);
            };

            let mut session_guard = session.lock().await;
            let progress = session_guard.resource_mut_progress(self.resource_id).expect("Progress not found");

            if progress.status == TransferStatus::Canceled {
                return Err(DataChannelError::SessionCanceled);
            }

            progress.update_progress(bytes.len() as u64);
            let _ = progress_sender
                .send(MessageToShell::HandleResponse(
                    core_request_id,
                    CoreOperationOutput::Transfer(TransferOperationOutput::TransferResourceProgressUpdate(progress.clone()))
                ))
                .await;

            let is_completed = progress.status.is_completed();
            drop(session_guard);

            if let Some(handle) = last_sent_handle.take() {
                _ = handle.await.map_err(|_| DataChannelError::DataCorrupted)??;
            }

            let throughput_controller = self.throughput_controller.clone();
            let data_channel = self.data_channel.clone();
            last_sent_handle = Some(spawn(async move {
                let sent_bytes = throughput_controller.send(Arc::downgrade(&data_channel), &bytes).await?;
                if sent_bytes < bytes.len() {
                    log::warn!(target: "nearby", "The data sent less than data read");
                    Err(DataChannelError::DataCorrupted)
                } else {
                    Ok(sent_bytes)
                }
            }));

            if is_completed {
                break;
            }
        }

        if let Some(handle) = last_sent_handle.take() {
            _ = handle.await.map_err(|_| DataChannelError::DataCorrupted)??;
        }

        let Some(session) = self.session.upgrade() else {
            return Err(DataChannelError::SessionCanceled);
        };

        let mut session_guard = session.lock().await;
        let progress = session_guard.resource_mut_progress(self.resource_id).expect("Progress not found");

        if !progress.is_completed() {
            return Err(DataChannelError::DataCorrupted);
        }

        Ok(())
    }
}

impl Drop for DataChannel {
    fn drop(&mut self) {
        let auto_close = self.auto_close.load(Ordering::Relaxed);
        if auto_close {
            let channel = self.data_channel.clone();
            let _ = spawn(async move {
                log::info!(target: "nearby", "Auto closing data channel: {}", channel.label());
                let _ = channel.close().await;
            });
        }
    }
}

impl Deref for DataChannel {
    type Target = RTCDataChannel;

    fn deref(&self) -> &Self::Target {
        &self.data_channel
    }
}

pub struct RTCStreamChannel {
    receiver: mpsc::Receiver<Result<Vec<u8>, DataChannelError>>,
    sender: Arc<mpsc::Sender<Result<Vec<u8>, DataChannelError>>>,
    data_channel: Arc<RTCDataChannel>
}

impl Stream for RTCStreamChannel {
    type Item = Result<Vec<u8>, DataChannelError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.receiver.poll_recv(cx)
    }
}

impl RTCStreamChannel {
    pub fn new(data_channel: Arc<RTCDataChannel>) -> Self {
        let (message_sender, message_receiver) = mpsc::channel::<Result<Vec<u8>, DataChannelError>>(2048);
        let message_sender = Arc::new(message_sender).clone();

        let maybe_sender = Arc::downgrade(&message_sender);
        data_channel.on_message(Box::new(move |message| {
            let maybe_sender = maybe_sender.clone();
            Box::pin(async move {
                if let Some(sender) = maybe_sender.upgrade() {
                    if let Err(e) = sender.send(Ok(message.data.to_vec())).await {
                        log::error!(target: "nearby", "Failed to send message: {:?}", e);
                    }
                }
            })
        }));

        let maybe_sender = Arc::downgrade(&message_sender);
        data_channel.on_close(Box::new(move || {
            let maybe_sender = maybe_sender.clone();
            Box::pin(async move {
                if let Some(sender) = maybe_sender.upgrade() {
                    let _ = sender.send(Err(DataChannelError::DataChannelClosed("The channel is closed".to_owned()))).await;
                }
            })
        }));

        let maybe_sender = Arc::downgrade(&message_sender);
        data_channel.on_error(Box::new(move |_err| {
            let maybe_sender = maybe_sender.clone();
            Box::pin(async move {
                if let Some(sender) = maybe_sender.upgrade() {
                    let _ = sender.send(Err(DataChannelError::DataChannelClosed(format!("{_err:?}")))).await;
                }
            })
        }));

        Self {
            receiver: message_receiver,
            data_channel,
            sender: message_sender
        }
    }

    pub async fn close(&self) -> bool {
        self.sender.closed().await;
        self.data_channel.close().await.is_ok()
    }
}

pub trait IntoRTCStream {
    fn into_stream(self) -> RTCStreamChannel;
}

impl IntoRTCStream for Arc<RTCDataChannel> {
    fn into_stream(self) -> RTCStreamChannel {
        RTCStreamChannel::new(self)
    }
}
