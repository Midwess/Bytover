use crate::app::file_system::file::{LocalResource, LocalResourcePath};
use crate::app::operations::transfer::TransferOperationOutput;
use crate::app::operations::CoreOperationOutput;
use crate::app::transfer::session::TransferProgress;
use crate::native::message_to_shell::MessageToShell;
use crate::{serialize, ShellRuntime};
use bytes::Bytes;
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
    #[error("Data channel is closed")]
    DataChannelClosed(String),
    #[error("Data corrupted")]
    DataCorrupted,
    #[error("Timeout")]
    Timeout(Duration)
}

#[derive(Clone)]
pub struct DataChannel {
    data_channel: Arc<RTCDataChannel>,
    shell_runtime: Arc<dyn ShellRuntime>,
    throughput_controller: Arc<ThroughputController>,
    pub resource_id: u64
}

impl DataChannel {
    pub fn data_label(resource_id: u64, session_id: u64) -> String {
        format!("{}-{}", resource_id, session_id)
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
        throughput_controller: Arc<ThroughputController>
    ) -> Result<Self, DataChannelError> {
        let label = data_channel.label().to_owned();
        let (resource_id, _) = DataChannel::from_label(&label).map_err(|e| DataChannelError::InvalidLabelFormat(label.clone()))?;

        throughput_controller.handle(Arc::downgrade(&data_channel)).await;
        Ok(Self {
            data_channel,
            shell_runtime,
            throughput_controller,
            resource_id
        })
    }

    pub async fn stream_resource(
        local_resource: &LocalResource,
        session_id: u64,
        connection: &ConnectionWebRtc,
        shell_runtime: Arc<dyn ShellRuntime>,
        throughput_controller: Arc<ThroughputController>
    ) -> Result<Self, DataChannelError> {
        let label = DataChannel::data_label(local_resource.order_id, session_id);
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

        throughput_controller.handle(Arc::downgrade(&data_channel)).await;

        log::info!(target: "nearby", "Data channel created: {}", label);
        Ok(Self {
            data_channel,
            shell_runtime,
            throughput_controller,
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

        let file = File::new(None, saved_path.clone()).await.map_err(|e| DataChannelError::FileError(e.to_string()))?;
        let mut file = file.open().await.map_err(|e| DataChannelError::FileError(e.to_string()))?;

        let mut received_bytes = 0;
        log::info!(target: "nearby", "Start downloading file into: {}", saved_path);
        let result = loop {
            let next_bytes = match self.throughput_controller.next_bytes(&mut stream).await {
                Ok(Some(bytes)) => bytes,
                Ok(None) => match received_bytes > file_size as usize {
                    true => break Ok(()),
                    false => break Err(DataChannelError::DataCorrupted)
                },
                Err(e) => break Err(e)
            };

            let written_bytes = file.write(&next_bytes).await.map_err(|e| DataChannelError::FileError(e.to_string()))?;

            if written_bytes < next_bytes.len() {
                break Err(DataChannelError::DataCorrupted);
            }

            received_bytes += written_bytes;

            self.update_progress(
                core_request_id,
                TransferProgress::progress(self.resource_id, received_bytes as f64 / file_size as f64)
            );

            if received_bytes >= file_size as usize {
                break Ok(());
            }
        };

        log::info!(target: "nearby", "Received bytes final: {} vs {}", received_bytes, file_size);

        let percentage = received_bytes as f64 / file_size as f64;
        if let Err(e) = result {
            self.update_progress(
                core_request_id,
                TransferProgress::fail(self.resource_id, percentage, e.to_string())
            );
        } else {
            self.update_progress(core_request_id, TransferProgress::progress(self.resource_id, percentage));
        }

        Ok(())
    }

    pub fn update_progress(&self, core_request_id: u32, progress: TransferProgress) -> JoinHandle<()> {
        let runtime = self.shell_runtime.clone();
        runtime.msg_from_native_bg(serialize(&MessageToShell::HandleResponse(
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
        let mut cursor = file.cursor(0, 64 * 1024 - 1).await.map_err(|e| DataChannelError::FileError(e.to_string()))?;

        log::info!(target: "nearby", "Start uploading file: {}", saved_path);
        let mut last_sent_handle: Option<JoinHandle<Result<usize, DataChannelError>>> = None;
        let mut total_sent = 0;
        loop {
            let bytes = cursor.next().await.map_err(|e| DataChannelError::FileError(format!("{:?}", e)))?;
            if let Some(handle) = last_sent_handle {
                let _ = handle.await.map_err(|_| DataChannelError::DataCorrupted)??;
            }

            let Some(bytes) = bytes.map(Bytes::from) else {
                break;
            };

            total_sent += bytes.len();
            let data_channel = self.data_channel.clone();
            let throughput_controller = self.throughput_controller.clone();
            last_sent_handle = Some(spawn(async move {
                let sent_bytes = throughput_controller.send(Arc::downgrade(&data_channel), &bytes).await?;
                if sent_bytes < bytes.len() {
                    Err(DataChannelError::DataCorrupted)
                } else {
                    Ok(sent_bytes)
                }
            }));

            self.update_progress(
                core_request_id,
                TransferProgress::progress(self.resource_id, total_sent as f64 / file_size as f64)
            );
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
                    let _ = sender.send(Ok(message.data.to_vec())).await;
                }
            })
        }));

        let maybe_sender = Arc::downgrade(&message_sender);
        data_channel.on_close(Box::new(move || {
            let maybe_sender = maybe_sender.clone();
            Box::pin(async move {
                if let Some(sender) = maybe_sender.upgrade() {
                    let _ = sender.send(Err(DataChannelError::DataChannelClosed("".to_owned()))).await;
                }
            })
        }));

        let maybe_sender = Arc::downgrade(&message_sender);
        data_channel.on_error(Box::new(move |_err| {
            let maybe_sender = maybe_sender.clone();
            Box::pin(async move {
                if let Some(sender) = maybe_sender.upgrade() {
                    let _ = sender.send(Err(DataChannelError::DataChannelClosed(format!("{:?}", _err)))).await;
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
