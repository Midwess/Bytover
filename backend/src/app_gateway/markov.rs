use async_trait::async_trait;
use thiserror::Error;
use tonic::Status;

#[derive(Debug, Error)]
pub enum MarkovErrors {
    #[error("Connection error: {0}")]
    ConnectionError(#[from] tonic::transport::Error),
    #[error("Generation error: {0}")]
    GenerationError(#[from] Status)
}

#[async_trait]
pub trait Markov: Send + Sync {
    async fn generate_name(&self) -> Result<String, MarkovErrors>;
}
