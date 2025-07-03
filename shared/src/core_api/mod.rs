pub mod network;

use crate::app::operations::transfer::TransferOperationOutput;
use crate::app::operations::CoreOperationOutput;
use crate::app::transfer::session::TransferProgress;
pub use core_services::local_storage::abstraction::IOCursor as IOReader;
use futures::channel::mpsc::UnboundedReceiver;
use matchbox_socket::PeerBuffered;
use n0_future::StreamExt;
use tokio::task::JoinHandle;
use url::Url;

#[derive(Debug, thiserror::Error)]
pub enum IOWriterError {
    #[error("IOWriter Error: {0}")]
    Error(String)
}

#[async_trait::async_trait]
pub trait IOWriter: Send + Sync {
    async fn write(&mut self, data: bytes::Bytes) -> anyhow::Result<()>;
}

#[async_trait::async_trait]
pub trait CoreBridge: Send + Sync {
    fn response(&self, request_id: u32, response: CoreOperationOutput) -> JoinHandle<()>;

    // How many throttle is depends on the implementation
    async fn response_throttle(&self, request_id: u32, response: CoreOperationOutput);

    async fn resource_progress_update(&self, request_id: u32, progress: &TransferProgress, is_sync: bool) {
        let response = CoreOperationOutput::Transfer(TransferOperationOutput::TransferResourceProgressUpdate(progress.clone()));
        if is_sync {
            let _ = self.response(request_id, response).await;
        } else {
            self.response_throttle(request_id, response).await;
        }
    }
}

// Abstraction open stream to http server
#[async_trait::async_trait]
pub trait NetStream: Send + Sync {
    async fn start(&self, http_url: Url, size: u64) -> anyhow::Result<Box<dyn NetStreamInner>>;
}

#[async_trait::async_trait]
pub trait NetStreamInner: Send + Sync {
    async fn write(&mut self, data: bytes::Bytes) -> anyhow::Result<()>;

    async fn end(&mut self) -> anyhow::Result<()>;
}

#[async_trait::async_trait]
pub trait TimeoutReceiver<T: Send + Sync>: Send + Sync {
    async fn recv_timeout(&mut self, timeout: std::time::Duration) -> Option<T>;
    async fn recv_default_timeout(&mut self) -> Option<T>;
}

#[async_trait::async_trait]
impl<T: Send + Sync> TimeoutReceiver<T> for UnboundedReceiver<T> {
    async fn recv_timeout(&mut self, timeout: std::time::Duration) -> Option<T> {
        tokio::time::timeout(timeout, self.next()).await.unwrap_or_else(|_| None)
    }

    async fn recv_default_timeout(&mut self) -> Option<T> {
        self.recv_timeout(std::time::Duration::from_secs(10)).await
    }
}

#[async_trait::async_trait]
pub trait BufferExt: Send + Sync {
    async fn flush_timeout(&self, index: usize) -> anyhow::Result<()>;
    async fn flush_all_timeout(&self) -> anyhow::Result<()>;
}

#[async_trait::async_trait]
impl BufferExt for PeerBuffered {
    async fn flush_timeout(&self, index: usize) -> anyhow::Result<()> {
        let buffered = self.buffered_amount(index).await;
        let timeout = std::time::Duration::from_secs((buffered / 40).min(10) as u64);
        Ok(tokio::time::timeout(timeout, self.flush(index)).await?)
    }

    async fn flush_all_timeout(&self) -> anyhow::Result<()> {
        let buffered = self.sum_buffered_amount().await;
        let timeout = std::time::Duration::from_secs((buffered / 40).min(10) as u64);
        Ok(tokio::time::timeout(timeout, self.flush_all()).await?)
    }
}
