use schema::value::static_resource::StaticResource;
use crate::entities::transfer_session::TransferSession;

#[derive(Debug, thiserror::Error)]
pub enum CloudStorageErrors {
    #[error("Cloud storage error {0}")]
    S3Errors(#[from] core_services::services::errors::Errors)
}

#[async_trait::async_trait]
pub trait CloudStorage: Send + Sync {
    async fn sign_upload(&self, source: &mut StaticResource) -> Result<String, CloudStorageErrors>;
    async fn sign_download(&self, source: &mut StaticResource) -> Result<String, CloudStorageErrors>;
}
