use std::time::Duration;

use crate::cloud_storage::storage::{CloudStorage, CloudStorageErrors};
use core_services::s3::S3Client;
use schema::value::static_resource::StaticResource;

pub struct S3CloudStorageImpl {
    pub s3_client: S3Client
}

#[async_trait::async_trait]
impl CloudStorage for S3CloudStorageImpl {
    async fn sign(&self, resource: &mut StaticResource) -> Result<String, CloudStorageErrors> {
        let duration = Duration::from_secs(60 * 60 * 24 * 3);
        let url = self.s3_client.sign_object(resource, duration).await?;

        Ok(url)
    }
}
