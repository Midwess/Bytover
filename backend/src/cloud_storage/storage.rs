use schema::value::static_resource::StaticResource;

use crate::entities::transfer_resource::TransferResource;

#[derive(Debug, thiserror::Error)]
pub enum CloudStorageErrors {
    #[error("Cloud storage error {0}")]
    S3Errors(#[from] core_services::services::errors::Errors)
}

#[async_trait::async_trait]
pub trait CloudStorage: Send + Sync {
    async fn sign_resource(&self, resource: &mut TransferResource) -> Result<(), CloudStorageErrors>;
    async fn sign(&self, source: &mut StaticResource) -> Result<(), CloudStorageErrors>;
}
