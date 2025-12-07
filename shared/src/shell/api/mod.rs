pub mod network;

use crate::app::operations::CoreOperationOutput;
use crate::app::AppEvent;
use crate::entities::local_resource::LocalResourcePath;
pub use core_services::local_storage::stream::IOCursor as IOReader;
use core_services::utils::cancellation::{CancellationToken, CancellationTokenExt, FutureExtension};
use crux_core::RequestHandle;
use futures::channel::mpsc::{Receiver, UnboundedReceiver};
use futures::task::{noop_waker, Context, Poll};
use futures_util::lock::Mutex;
use matchbox_socket::PeerBuffered;
use n0_future::Stream;
use schema::devlog::bitbridge::client_upload_request::Upload;
use schema::devlog::bitbridge::MultiPartUploadComplete;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use bytes::Bytes;
use core_services::local_storage::stream::IOCursor;
use crate::utils::compression::CompressStats;

#[derive(Debug, thiserror::Error)]
pub enum IOWriterError {
    #[error("IOWriter Error: {0}")]
    Error(String)
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
pub trait IOWriter: Send + Sync {
    async fn write(&mut self, data: bytes::Bytes) -> anyhow::Result<usize>;
    async fn flush(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
    async fn end(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
}

#[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
#[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
pub trait CIOCursor: IOCursor {
    // Return a compressed chunk
    // and the raw size of the chunk before compression
    // bandwidth is in bytes/sec
    async fn c_next(&mut self, max: Option<u64>) -> anyhow::Result<Option<(&[u8], usize)>>;
    fn compression_stats_mut(&mut self) -> &mut CompressStats;
    fn update_bandwidth(&mut self, network: f64) -> bool;
    fn update_should_compress(&mut self, should_compress: bool);
}

#[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
#[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
pub trait DIOWriter: IOWriter {
    /// Receive a compressed chunk
    /// return an amount data that written (uncompressed size)
    async fn d_write(&mut self, data: Bytes) -> anyhow::Result<Option<usize>>;
}

pub enum CruxRequest {
    Id(u32),
    RequestHandle(RequestHandle<CoreOperationOutput>)
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
pub trait CoreBridge: Send + Sync {
    async fn response(&self, request: &mut CruxRequest, response: CoreOperationOutput);

    // How many throttle is depends on the implementation
    async fn response_throttle(&self, request: &mut CruxRequest, response: CoreOperationOutput);

    async fn notify(&self, event: AppEvent);
}

pub struct CoreRequest {
    crux_request: Arc<Mutex<CruxRequest>>,
    bridge: &'static dyn CoreBridge
}

impl Clone for CoreRequest {
    fn clone(&self) -> Self {
        Self {
            crux_request: self.crux_request.clone(),
            bridge: self.bridge
        }
    }
}

impl CoreRequest {
    pub fn new(crux_request: CruxRequest, bridge: &'static dyn CoreBridge) -> Self {
        Self {
            crux_request: Arc::new(Mutex::new(crux_request)),
            bridge
        }
    }

    pub async fn response(&self, response: impl Into<CoreOperationOutput>) {
        let mut request = self.crux_request.lock().await;
        self.bridge.response(&mut request, response.into()).await;
    }

    pub async fn response_throttle(&self, response: impl Into<CoreOperationOutput>) {
        let mut request = self.crux_request.lock().await;
        let _ = self.bridge.response_throttle(&mut request, response.into()).await;
    }
}

#[derive(Debug)]
pub enum NetStreamEvent {
    Progress { uploaded_bytes: u64 },
    Completed(Option<MultiPartUploadComplete>),
    Error(anyhow::Error)
}

// Abstraction open stream to http server
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
pub trait NetStream: Send + Sync {
    async fn upload_resource(&self, request: Upload, path: LocalResourcePath) -> anyhow::Result<Box<dyn NetStreamInner>>;
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
pub trait NetStreamInner: Send + Sync {
    // Upload the resource to url
    async fn start(&mut self) -> anyhow::Result<Receiver<NetStreamEvent>>;

    async fn end(&mut self) -> anyhow::Result<()>;
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
pub trait TimeoutReceiver<T: Send + Sync>: Send + Sync {
    fn poll_next_now(&mut self) -> Option<T>;
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
impl<T: Send + Sync> TimeoutReceiver<T> for UnboundedReceiver<T> {
    fn poll_next_now(&mut self) -> Option<T> {
        let waker = noop_waker();
        let mut cx = Context::from_waker(&waker);
        let mut pinned = Pin::new(self);
        match pinned.as_mut().poll_next(&mut cx) {
            Poll::Ready(Some(item)) => Some(item),
            _ => None
        }
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
pub trait BufferExt: Send + Sync {
    async fn flush_timeout(&self, index: usize) -> anyhow::Result<usize>;
    async fn flush_all_timeout(&self) -> anyhow::Result<()>;
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
impl BufferExt for PeerBuffered {
    async fn flush_timeout(&self, index: usize) -> anyhow::Result<usize> {
        let buffered = self.buffered_amount(index).await;
        if buffered == 0 {
            return Ok(0);
        }

        // Assume min speed = 10 KB/s = 10,000 bytes/s
        let est_secs = buffered as f64 / 10_000.0;

        // Clamp between 5s and 10s
        let secs = est_secs.clamp(5.0, 10.0);
        let timeout = Duration::from_secs_f64(secs);

        let cancel = CancellationToken::timeout(timeout);
        let flushed = self.flush(index).with_cancel(&cancel).await.is_ok();
        if flushed {
            let new_buffered = self.buffered_amount(index).await;
            return if new_buffered < buffered {
                Ok(buffered - new_buffered)
            } else {
                Err(anyhow::anyhow!("Peer hang up at {}", new_buffered))
            }
        }

        Ok(0)
    }

    async fn flush_all_timeout(&self) -> anyhow::Result<()> {
        for i in 0..self.len() {
            self.flush_timeout(i).await?;
        }

        Ok(())
    }
}
