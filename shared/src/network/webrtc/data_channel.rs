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
    CreateDataChannelError(#[from] webrtc::Error),
    #[error("Failed to open data channel")]
    OpenDataChannelError(String),
    #[error("File not exists")]
    FileError(String),
    #[error("Resource not found")]
    ResourceNotFound,
    #[error("Data channel is closed")]
    DataChannelClosed
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
        let label = data_channel.label().to_owned();
        let parts = label.split(":").collect::<Vec<&str>>();
        if parts.len() != 3 {
            return Err(DataChannelError::InvalidLabelFormat(label));
        }

        let (session_id, resource_id, file_name, file_size) =
            LocalResource::read_identifier(parts[0].to_string()).map_err(|_| DataChannelError::InvalidLabelFormat(label.clone()))?;

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

        let mut left_bytes = file_size;
        while let Some(Ok(bytes)) = stream.next().await {
            let recevied_bytes_len = bytes.len();
            file.write(&bytes).await.map_err(|e| DataChannelError::FileError(e.to_string()))?;
            left_bytes -= recevied_bytes_len as u64;

            let progress = TransferProgress {
                resource_order_id: self.resource_id,
                percentage: (file_size - left_bytes) as f32 / file_size as f32,
                status: TransferStatus::InProgress
            };

            self.update_progress(progress).await;
        }

        let progress = TransferProgress {
            resource_order_id: self.resource_id,
            percentage: if left_bytes == 0 {
                1.0
            } else {
                (file_size - left_bytes) as f32 / file_size as f32
            },
            status: TransferStatus::Success
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
        let mut cursor = file.cursor(0, 6 * 1024 * 1024).await.map_err(|e| DataChannelError::FileError(e.to_string()))?;
        let mut left_bytes = file_size;
        while let Ok(Some(bytes)) = cursor.next().await {
            let sent_bytes_len = bytes.len();
            self.data_channel.send(&bytes.into()).await?;
            left_bytes -= sent_bytes_len as u64;
            let progress = TransferProgress {
                resource_order_id: self.resource_id,
                percentage: (file_size - left_bytes) as f32 / file_size as f32,
                status: TransferStatus::InProgress
            };

            self.update_progress(progress).await;
        }

        let progress = TransferProgress {
            resource_order_id: self.resource_id,
            percentage: if left_bytes == 0 {
                1.0
            } else {
                (file_size - left_bytes) as f32 / file_size as f32
            },
            status: TransferStatus::Success
        };

        self.update_progress(progress).await;

        self.data_channel.close().await?;

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
        let (message_sender, message_receiver) = mpsc::channel::<Result<Vec<u8>, DataChannelError>>(100);

        let msg_sender = message_sender.clone();
        let data_channel_cloned = data_channel.clone();
        data_channel.on_message(Box::new(move |message| {
            let msg_sender = msg_sender.clone();
            Box::pin(async move {
                let _ = msg_sender.send(Ok(message.data.to_vec()));
            })
        }));

        let msg_sender = message_sender.clone();
        data_channel.on_close(Box::new(move || {
            let _ = msg_sender.send(Err(DataChannelError::DataChannelClosed));
            Box::pin(async move {})
        }));

        let msg_sender = message_sender.clone();
        data_channel.on_error(Box::new(move |_err| {
            let _ = msg_sender.send(Err(DataChannelError::DataChannelClosed));
            Box::pin(async move {})
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
