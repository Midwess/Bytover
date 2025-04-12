use crate::app::file_system::file::{LocalResource, LocalResourcePath};
use crate::app::operations::transfer::TransferOperationOutput;
use crate::app::operations::CoreOperationOutput;
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
    #[error("Data channel error {:?}", .0)]
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
    Timeout(#[from] tokio::time::error::Elapsed),
}

#[derive(Clone)]
pub struct DataChannel {
    data_channel: Arc<RTCDataChannel>,
    shell_runtime: Arc<dyn ShellRuntime>,
    pub resource_id: u64
}

impl DataChannel {
    pub fn from_channel(
        data_channel: Arc<RTCDataChannel>,
        shell_runtime: Arc<dyn ShellRuntime>
    ) -> Result<Self, DataChannelError> {
        data_channel.on_open(Box::new(move || {
            log::info!(target: "nearby", "Data channel opened");
            Box::pin(async move {})
        }));

        let label = data_channel.label().to_owned();

        let resource_id = label.parse::<u64>().map_err(|_| DataChannelError::InvalidLabelFormat(label.clone()))?;

        Ok(Self {
            data_channel,
            shell_runtime,
            resource_id
        })
    }

    pub async fn stream_resource(
        local_resource: &LocalResource,
        connection: &ConnectionWebRtc,
        shell_runtime: Arc<dyn ShellRuntime>
    ) -> Result<Self, DataChannelError> {
        let label = local_resource.order_id.to_string();
        log::info!(target: "nearby", "Creating data channel: {}", label);
        let data_channel = connection.peer_connection.create_data_channel(label.as_str(), Some(ConnectionWebRtc::channel_config())).await?;

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

        log::info!(target: "nearby", "Data channel created: {}", label);
        Ok(Self {
            data_channel,
            shell_runtime,
            resource_id: local_resource.order_id
        })
    }

    pub async fn stop_transfer(&self) {
        let _ = self.data_channel.close().await;
    }

    pub async fn start_download(&self, core_request_id: u32, out_resource: &LocalResource) -> Result<(), DataChannelError> {
        let mut stream = RTCStreamChannel::new(self.data_channel.clone());
        let file_size = out_resource.size;

        let saved_path = match &out_resource.path {
            LocalResourcePath::LocalPath(file_path) => file_path.clone(),
            LocalResourcePath::PlatformIdentifier(_) => {
                return Err(DataChannelError::FileError("Platform identifier is not supported".to_string()))
            }
        };

        // let file = File::new(None, saved_path.clone()).await.map_err(|e| DataChannelError::FileError(e.to_string()))?;
        // let mut file = file.open().await.map_err(|e| DataChannelError::FileError(e.to_string()))?;

        let mut received_bytes = 0;
        log::info!(target: "nearby", "Start downloading file into: {}", saved_path);
        while let Some(next_bytes) = timeout(Duration::from_secs(5), stream.next()).await? {
            let bytes = next_bytes?;
            // let written_bytes = file.write(&bytes).await.map_err(|e| DataChannelError::FileError(e.to_string()))?;

            received_bytes += bytes.len(); 

            // self.update_progress(
            //     core_request_id,
            //     TransferProgress::progress(self.resource_id, received_bytes as f64 / file_size as f64)
            // );

            if received_bytes >= file_size as usize {
                break;
            }
        }

        log::info!(target: "nearby", "Received bytes final: {} vs {}", received_bytes, file_size);

        if received_bytes < file_size as usize {
            Err(DataChannelError::DataCorrupted)
        } else {
            Ok(())
        }
    }

    pub fn update_progress(&self, core_request_id: u32, progress: TransferProgress) -> JoinHandle<()> {
        let runtime = self.shell_runtime.clone();
        runtime
            .msg_from_native_bg(serialize(&MessageToShell::HandleResponse(
                core_request_id,
                CoreOperationOutput::Transfer(TransferOperationOutput::TransferResourceProgressUpdate(progress))
            )))
    }

    pub async fn start_upload(&self, core_request_id: u32, resource: LocalResource) -> Result<(), DataChannelError> {
        let saved_path = match &resource.path {
            LocalResourcePath::LocalPath(file_path) => file_path.clone(),
            LocalResourcePath::PlatformIdentifier(_) => {
                return Err(DataChannelError::FileError("Platform identifier is not supported".to_string()))
            }
        };

        let file = File::existing(saved_path.clone()).await.map_err(|e| DataChannelError::FileError(e.to_string()))?;
        let file_size = resource.size;
        let mut cursor = file.cursor(0, 32 * 1024).await.map_err(|e| DataChannelError::FileError(e.to_string()))?;

        log::info!(target: "nearby", "Start uploading file: {}", saved_path);
        let mut last_sent_handle: Option<JoinHandle<Result<usize, DataChannelError>>> = None;
        let mut total_sent = 0;
        loop {
            let bytes = cursor.next().await.map_err(|e| DataChannelError::FileError(format!("{:?}", e)))?;
            if let Some(handle) = last_sent_handle {
                let _ = handle.await.map_err(|_| DataChannelError::DataCorrupted)??;
            }

            let Some(bytes) = bytes else {
                break;
            };

            total_sent += bytes.len();
            let data_channel = self.data_channel.clone();
            let order_id = self.resource_id;
            last_sent_handle = Some(spawn(async move {
                let expected_bytes = bytes.len();
                let sent_bytes = timeout(Duration::from_secs(5), data_channel.send(&bytes.into())).await??;

                let curr_amount = data_channel.buffered_amount().await;
                log::info!(target: "nearby", "Buffered amount of {}: {}", order_id, curr_amount);

                if sent_bytes < expected_bytes {
                    Err(DataChannelError::DataCorrupted)
                } else {
                    Ok(sent_bytes)
                }
            }));

            // self.update_progress(
            //     core_request_id,
            //     TransferProgress::progress(self.resource_id, total_sent as f64 / file_size as f64)
            // );
        }

        if total_sent < file_size as usize {
            Err(DataChannelError::DataCorrupted)
        } else {
            Ok(())
        }
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
            // let _ = channel.close().await;
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
        let (message_sender, message_receiver) = mpsc::channel::<Result<Vec<u8>, DataChannelError>>(2048);

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
