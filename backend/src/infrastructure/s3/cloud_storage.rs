use crate::cloud_storage::storage::{CloudStorage, CloudStorageErrors, UploadContext};
use crate::entities::transfer_resource::TransferResource;
use core_services::s3::S3Client;
use schema::devlog::auth_gateway::models::User;
use schema::devlog::bitbridge::client_upload_request::Upload;
use schema::devlog::bitbridge::{MultiPartUpload, MultiPartUploadComplete};
use schema::value::static_resource::StaticResource;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::Instant;

pub struct S3CloudStorageImpl {
    pub s3_client: S3Client,
    pub cached_sign: Arc<Mutex<HashMap<StaticResource, (Instant, String)>>>
}

#[async_trait::async_trait]
impl CloudStorage for S3CloudStorageImpl {
    async fn get_upload_solution_for_resource(&self, user: &User, resource: &TransferResource) -> Result<Upload, CloudStorageErrors> {
        let file_size = Some(resource.size_in_bytes() as usize);
        let source = resource.source();
        log::info!("Get upload solution for resource: {:?} size: {:?}", source, file_size);
        self.get_upload_solution(user, &source, file_size).await
    }

    async fn get_upload_solution(
        &self,
        user: &User,
        source: &StaticResource,
        file_size: Option<usize>
    ) -> Result<Upload, CloudStorageErrors> {
        let duration = self.get_download_duration();
        let Some(file_size) = file_size else {
            let single_url = self.s3_client.sign_upload(source, duration).await?;
            return Ok(Upload::SingleUrl(single_url));
        };

        let upload_id = self.s3_client.create_multipart_upload(source).await?;
        let upload_url = self.s3_client.generate_part_upload_url(source, &upload_id, 1, self.get_download_duration()).await?;
        let context = UploadContext::new(user.id.clone(), upload_id, source.clone(), file_size as u64)?;
        let token = context.as_token(self.get_jwt_secret());
        let part = MultiPartUpload {
            context_token: token,
            upload_url
        };

        Ok(Upload::Multipart(part))
    }

    async fn complete_upload_part(&self, user: &User, context_token: &str) -> Result<Option<MultiPartUpload>, CloudStorageErrors> {
        let context = UploadContext::from_token(context_token, self.get_jwt_secret(), user)?;
        let Ok(next_part) = context.next() else { return Ok(None) };

        let part_url = self
            .s3_client
            .generate_part_upload_url(
                &next_part.resource,
                &next_part.upload_id,
                next_part.part_number,
                self.get_download_duration()
            )
            .await?;

        Ok(Some(MultiPartUpload {
            upload_url: part_url,
            context_token: next_part.as_token(self.get_jwt_secret())
        }))
    }

    async fn complete_upload(&self, user: &User, completion: &MultiPartUploadComplete) -> Result<(), CloudStorageErrors> {
        let context = UploadContext::from_token(&completion.context_token, self.get_jwt_secret(), user)?;

        self.s3_client
            .complete_multipart_upload(&context.resource, context.upload_id, completion.e_tags.clone())
            .await?;

        Ok(())
    }

    async fn generate_download_url(&self, source: &StaticResource) -> Result<String, CloudStorageErrors> {
        let duration = self.get_download_duration();
        let mut cached_sign = self.cached_sign.lock().await;
        if let Some((since, signed)) = cached_sign.get_mut(source) {
            if since.elapsed() < duration / 2 {
                return Ok(signed.clone())
            }
        }

        drop(cached_sign);

        let url = self.s3_client.sign_download(source, duration).await?;

        self.cached_sign.lock().await.insert(source.clone(), (Instant::now(), url.clone()));

        Ok(url)
    }
}
