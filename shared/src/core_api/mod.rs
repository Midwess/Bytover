use tokio::task::JoinHandle;
use url::Url;
pub use core_services::local_storage::abstraction::IOCursor as IOReader;
use crate::app::operations::CoreOperationOutput;

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
