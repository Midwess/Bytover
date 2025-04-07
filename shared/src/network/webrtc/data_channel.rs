use crate::app::file_system::file::{LocalResource, LocalResourcePath};
use crate::app::transfer::session::{TransferProgress, TransferStatus};
use crate::native::message_to_shell::MessageToShell;
use crate::{serialize, ShellRuntime};
use core_services::local_storage::file_system::File;
use futures_util::{Stream, StreamExt};
use std::ops::Deref;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::spawn;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tokio::time::timeout;
use webrtc::data_channel::RTCDataChannel;

use super::connection::ConnectionWebRtc;
use std::pin::Pin;
use std::task::{Context, Poll};
use thiserror::Error;
use tokio::sync::mpsc;

#[derive(Error, Debug)]
pub enum DataChannelError {
    #[error("Invalid format of data channel label")]
    InvalidLabelFormat(String),
    #[error("Failed to create data channel {:?}", .0)]
    WebRtcError(#[from] webrtc::Error),
    #[error("Failed to open data channel")]
    OpenDataChannelError(String),
    #[error("File not exists")]
    FileError(String),
    #[error("Resource not found")]
    ResourceNotFound,
    #[error("Data channel is closed")]
    DataChannelClosed,
    #[error("Data corrupted")]
    DataCorrupted,
    #[error("Timeout")]
    Timeout(#[from] tokio::time::error::Elapsed)
}

#[derive(Clone)]
pub struct DataChannel {
    data_channel: Arc<RTCDataChannel>,
    shell_runtime: Arc<dyn ShellRuntime>,
    pub session_id: u64,
    pub resource_id: u64,
    pub file_name: String,
    pub file_size: u64,
    pub workdir: String,
    pub saved_path: String
}

impl DataChannel {
    pub async fn from_channel(
        data_channel: Arc<RTCDataChannel>,
        shell_runtime: Arc<dyn ShellRuntime>,
        workdir: String
    ) -> Result<Self, DataChannelError> {
        data_channel.on_open(Box::new(move || {
            log::info!(target: "nearby", "Data channel opened");
            Box::pin(async move {})
        }));

        let label = data_channel.label().to_owned();

        let (session_id, resource_id, file_name, file_size) =
            LocalResource::read_identifier(label.clone()).map_err(|_| DataChannelError::InvalidLabelFormat(label.clone()))?;

        let saved_path = format!("{}/sessions/{}/{}/{}", workdir, session_id, resource_id, file_name);

        Ok(Self {
            data_channel,
            shell_runtime,
            session_id,
            resource_id,
            file_name,
            file_size,
            workdir,
            saved_path
        })
    }

    pub async fn stream_resource(
        session_id: u64,
        local_resource: LocalResource,
        connection: &ConnectionWebRtc,
        shell_runtime: Arc<dyn ShellRuntime>,
        workdir: String
    ) -> Result<Self, DataChannelError> {
        let label = local_resource.identifer(session_id);
        log::info!(target: "nearby", "Creating data channel: {}", label);
        let data_channel = connection.peer_connection.create_data_channel(label.as_str(), None).await?;

        let (open_sender, open_receiver) = oneshot::channel();
        data_channel.on_open(Box::new(move || {
            let _ = open_sender.send(());
            Box::pin(async move {})
        }));

        let Ok(_) = timeout(Duration::from_secs(10), open_receiver).await else {
            return Err(DataChannelError::OpenDataChannelError(format!(
                "Data channel {} is not open",
                label
            )));
        };

        let saved_path = match &local_resource.path {
            LocalResourcePath::LocalPath(file_path) => file_path.clone(),
            LocalResourcePath::PlatformIdentifier(_) => {
                return Err(DataChannelError::FileError("Platform identifier is not supported".to_string()))
            }
        };

        log::info!(target: "nearby", "Data channel created: {}", label);
        Ok(Self {
            data_channel,
            shell_runtime,
            session_id,
            resource_id: local_resource.order_id,
            file_name: local_resource.name,
            file_size: local_resource.size,
            workdir,
            saved_path
        })
    }

    pub async fn stop_transfer(&self) {
        let _ = self.data_channel.close().await;
    }

    pub async fn start_download(&self) -> Result<(), DataChannelError> {
        let mut stream = RTCStreamChannel::new(self.data_channel.clone());
        let file_size = self.file_size;

        let file = File::new(None, self.saved_path.clone())
            .await
            .map_err(|e| DataChannelError::FileError(e.to_string()))?;
        let mut file = file.open().await.map_err(|e| DataChannelError::FileError(e.to_string()))?;

        let mut received_bytes = 0;
        log::info!(target: "nearby", "Start downloading file into: {}", self.saved_path);
        while let Ok(Some(Ok(bytes))) = timeout(Duration::from_secs(5), stream.next()).await {
            let written_bytes = match file.write(&bytes).await.map_err(|e| DataChannelError::FileError(e.to_string())) {
                Ok(written_bytes) => written_bytes,
                Err(e) => {
                    log::error!(target: "nearby", "Failed to write data, stop transfer: {:?}", e);
                    break;
                }
            };

            received_bytes += written_bytes;
            if received_bytes == file_size as usize {
                break;
            }
        }

        let progress = TransferProgress {
            resource_order_id: self.resource_id,
            percentage: 1.0,
            status: if received_bytes == file_size as usize {
                TransferStatus::Success
            } else {
                TransferStatus::Fail
            }
        };

        self.update_progress(progress).await;

        let _ = stream.close().await;

        Ok(())
    }

    pub async fn update_progress(&self, progress: TransferProgress) {
        self.shell_runtime
            .msg_from_native(serialize(&MessageToShell::SessionProgress(self.session_id, progress)));
    }

    pub async fn start_upload(&self) -> Result<(), DataChannelError> {
        let file = File::existing(self.saved_path.clone())
            .await
            .map_err(|e| DataChannelError::FileError(e.to_string()))?;
        let file_size = self.file_size;
        let mut cursor = file.cursor(0, 16 * 1024).await.map_err(|e| DataChannelError::FileError(e.to_string()))?;

        log::info!(target: "nearby", "Start uploading file: {}", self.saved_path);
        let mut last_sent_handle: Option<JoinHandle<Result<usize, DataChannelError>>> = None;
        let mut total_sent = 0;
        while let Ok(bytes) = cursor.next().await {
            if let Some(handle) = last_sent_handle {
                let sent_bytes = match handle.await {
                    Ok(Ok(sent_bytes)) => {
                        sent_bytes
                    }
                    other => {
                        log::error!(target: "nearby", "Failed to send data, stop transfer {:?}", other);
                        break;
                    }
                };

                total_sent += sent_bytes;
            }

            let Some(bytes) = bytes else {
                break;
            };

            let data_channel = self.data_channel.clone();
            last_sent_handle = Some(spawn(async move {
                let expected_bytes = bytes.len();
                let sent_bytes = timeout(Duration::from_secs(5), data_channel.send(&bytes.into())).await??;

                if sent_bytes < expected_bytes {
                    Err(DataChannelError::DataCorrupted)
                } else {
                    Ok(sent_bytes)
                }
            }));
        }

        let progress = TransferProgress {
            resource_order_id: self.resource_id,
            percentage: 1.0,
            status: if total_sent == file_size as usize {
                TransferStatus::Success
            } else {
                TransferStatus::Fail
            }
        };

        self.update_progress(progress).await;

        Ok(())
    }
}

impl Deref for DataChannel {
    type Target = RTCDataChannel;

    fn deref(&self) -> &Self::Target {
        &self.data_channel
    }
}

impl Drop for DataChannel {
    fn drop(&mut self) {
        let channel = self.data_channel.clone();
        spawn(async move {
            let _ = channel.close().await;
        });
    }
}

pub struct RTCStreamChannel {
    receiver: mpsc::Receiver<Result<Vec<u8>, DataChannelError>>,
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
        let (message_sender, message_receiver) = mpsc::channel::<Result<Vec<u8>, DataChannelError>>(2048); // TODO: Handle buffer overflow

        let msg_sender = message_sender.clone();
        let data_channel_cloned = data_channel.clone();
        data_channel.on_message(Box::new(move |message| {
            let msg_sender = msg_sender.clone();
            Box::pin(async move {
                let _ = msg_sender.send(Ok(message.data.to_vec())).await;
            })
        }));

        let msg_sender = message_sender.clone();
        data_channel.on_close(Box::new(move || {
            let msg_sender = msg_sender.clone();
            Box::pin(async move {
                let _ = msg_sender.send(Err(DataChannelError::DataChannelClosed)).await;
            })
        }));

        let msg_sender = message_sender.clone();
        data_channel.on_error(Box::new(move |_err| {
            let msg_sender = msg_sender.clone();
            Box::pin(async move {
                let _ = msg_sender.send(Err(DataChannelError::DataChannelClosed)).await;
            })
        }));

        Self {
            receiver: message_receiver,
            data_channel: data_channel_cloned
        }
    }

    pub async fn close(&self) -> bool {
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
