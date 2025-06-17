use std::time::Duration;

use crate::cloud_storage::storage::{CloudStorage, CloudStorageErrors};
use crate::entities::transfer_resource::TransferResource;
use core_services::s3::S3Client;
use schema::value::static_resource::StaticResource;

pub struct S3CloudStorageImpl {
    pub s3_client: S3Client
}

#[async_trait::async_trait]
impl CloudStorage for S3CloudStorageImpl {
    async fn sign_resource(&self, resource: &mut TransferResource) -> Result<(), CloudStorageErrors> {
        let duration = Duration::from_secs(60 * 60 * 24 * 3);
        self.s3_client.sign_object(resource.source_mut(), duration).await?;

        Ok(())
    }

    async fn sign(&self, resource: &mut StaticResource) -> Result<(), CloudStorageErrors> {
        let duration = Duration::from_secs(60 * 60 * 24 * 3);
        self.s3_client.sign_object(resource, duration).await?;

        Ok(())
    }
}
