use async_trait::async_trait;
use thiserror::Error;
use tonic::Status;

#[derive(Debug, Error)]
pub enum MarkovErrors {
    ConnectionError(#[from] tonic::transport::Error),
    GenerationError(#[from] Status),
}

#[async_trait]
pub trait Markov {
    async fn generate_name(&self) -> Result<String, MarkovErrors>;
}