use crate::app::file_system::file::LocalResourcePath;
use crate::app::transfer::session::TransferSession;
use thiserror::Error;
use tokio::sync::mpsc;

#[derive(Debug, Clone, Error)]
pub enum TransferStreamError {
    #[error("Failed to prepare transfer")]
    PrepareFailed(String)
}

pub struct InBoundStream {
    pub key: LocalResourcePath,
    pub stream: mpsc::Receiver<Vec<u8>>
}

pub struct OutBoundStream {
    pub key: LocalResourcePath,
    pub start_position: usize,
    pub stream: mpsc::Sender<Vec<u8>>
}

#[async_trait::async_trait]
pub trait UpStream {
    async fn prepare(&self, session: &TransferSession) -> Result<Vec<OutBoundStream>, TransferStreamError>;
    async fn cancel(&self);
}

#[async_trait::async_trait]
pub trait DownStream {
    async fn prepare(&self, session: &TransferSession) -> Result<Vec<InBoundStream>, TransferStreamError>;
    async fn cancel(&self);
}
