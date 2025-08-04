use crate::cloud_storage::storage::{CloudStorage, CloudStorageErrors};
use core_services::s3::S3Client;
use schema::value::static_resource::StaticResource;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::Instant;

pub struct S3CloudStorageImpl {
    pub s3_client: S3Client,
    pub cached_sign: Arc<Mutex<HashMap<StaticResource, (Instant, String)>>>
}

#[async_trait::async_trait]
impl CloudStorage for S3CloudStorageImpl {
    async fn sign_upload(&self, resource: &mut StaticResource) -> Result<String, CloudStorageErrors> {
        let duration = Duration::from_secs(60 * 60 * 24 * 3);
        let url = self.s3_client.sign_upload(resource, duration).await?;

        Ok(url)
    }

    async fn sign_download(&self, resource: &mut StaticResource) -> Result<String, CloudStorageErrors> {
        let duration = Duration::from_secs(60 * 60 * 24 * 4);
        let mut cached_sign = self.cached_sign.lock().await;
        if let Some((since, signed)) = cached_sign.get_mut(resource) {
            if since.elapsed() < duration / 2 {
                return Ok(signed.clone())
            }
        }

        drop(cached_sign);

        let url = self.s3_client.sign_download(resource, duration).await?;

        self.cached_sign.lock().await.insert(resource.clone(), (Instant::now(), url.clone()));

        Ok(url)
    }
}
