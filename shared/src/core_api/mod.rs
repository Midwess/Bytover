pub mod network;

use std::collections::HashMap;
use crate::app::operations::transfer::TransferOperationOutput;
use crate::app::operations::CoreOperationOutput;
use crate::app::transfer::session::TransferProgress;
use crate::app::AppEvent;
use crate::entities::file_system::file::LocalResourcePath;
pub use core_services::local_storage::stream::IOCursor as IOReader;
use futures::channel::mpsc::UnboundedReceiver;
use futures::task::{noop_waker, Context, Poll};
use futures_timer::Delay;
use futures_util::{select, FutureExt};
use matchbox_socket::PeerBuffered;
use n0_future::task::JoinHandle;
use n0_future::{Stream, StreamExt};
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;
use url::Url;

#[derive(Debug, thiserror::Error)]
pub enum IOWriterError {
    #[error("IOWriter Error: {0}")]
    Error(String)
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
pub trait IOWriter: Send + Sync {
    async fn write(&mut self, data: bytes::Bytes) -> anyhow::Result<()>;
    async fn flush(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
    async fn end(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
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

    async fn notify(&self, event: AppEvent);
}

#[derive(Debug)]
pub enum NetStreamEvent {
    Progress { uploaded_bytes: u64 },
    Completed(Vec<UploadResponse>),
    Error(anyhow::Error)
}

#[derive(Debug, Clone)]
pub struct UploadResponse {
    pub headers: HashMap<String, String>,
    pub json: Option<serde_json::Value>
}

#[derive(Debug, Clone)]
pub struct UploadRequest {
    pub url: Url,
    pub x_content_length: u64,
}

// Abstraction open stream to http server
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
pub trait NetStream: Send + Sync {
    async fn upload_resource(&self, requests: Vec<UploadRequest>, path: LocalResourcePath) -> anyhow::Result<Box<dyn NetStreamInner>>;
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
pub trait NetStreamInner: Send + Sync {
    // Upload the resource to url
    async fn start(&mut self) -> anyhow::Result<UnboundedReceiver<NetStreamEvent>>;

    async fn end(&mut self) -> anyhow::Result<()>;
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
pub trait TimeoutReceiver<T: Send + Sync>: Send + Sync {
    async fn recv_timeout(&mut self, timeout: Duration) -> Option<T>;
    async fn recv_default_timeout(&mut self) -> Option<T>;
    fn poll_next_now(&mut self) -> Option<T>;
    async fn recv_with_abort_signal<F, Fut>(&mut self, abort: F) -> Option<T>
    where
        F: FnOnce() -> Fut + Send,
        Fut: Future<Output = ()> + Send
    {
        let recv_fn = self.recv_default_timeout();
        select! {
            _ = abort().fuse() => None,
            res = recv_fn.fuse() => res,
            complete => None,
        }
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
impl<T: Send + Sync> TimeoutReceiver<T> for UnboundedReceiver<T> {
    async fn recv_timeout(&mut self, timeout: Duration) -> Option<T> {
        select! {
            msg = futures::StreamExt::next(self).fuse() => msg,
            _ = Delay::new(timeout).fuse() => None,
            complete => None,
        }
    }

    async fn recv_default_timeout(&mut self) -> Option<T> {
        self.recv_timeout(Duration::from_secs(10)).await
    }

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
    async fn flush_timeout(&self, index: usize) -> anyhow::Result<()>;
    async fn flush_all_timeout(&self) -> anyhow::Result<()>;
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
impl BufferExt for PeerBuffered {
    async fn flush_timeout(&self, index: usize) -> anyhow::Result<()> {
        let buffered = self.buffered_amount(index).await;

        // Assume min speed = 10 KB/s = 10,000 bytes/s
        let est_secs = buffered as f64 / 10_000.0;

        // Clamp between 5s and 10s
        let secs = est_secs.clamp(5.0, 10.0);
        let timeout = Duration::from_secs_f64(secs);

        select! {
            _ = self.flush(index).fuse() => Ok(()),
            _ = Delay::new(timeout).fuse() => Err(anyhow::anyhow!("flush timeout after {:?}", timeout)),
            complete => Ok(()),
        }
    }

    async fn flush_all_timeout(&self) -> anyhow::Result<()> {
        for i in 0..self.len() {
            self.flush_timeout(i).await?;
        }

        Ok(())
    }
}
