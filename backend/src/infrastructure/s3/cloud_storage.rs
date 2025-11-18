use crate::cloud_storage::storage::{CloudStorage, CloudStorageErrors, UploadContext, GB, MB};
use crate::entities::transfer_resource::{TransferResource, TransferResourceType};
use core_services::s3::S3Client;
use schema::devlog::app_gateway::models::User;
use schema::devlog::bitbridge::client_upload_request::Upload;
use schema::devlog::bitbridge::{MultiPartUpload, MultiPartUploadComplete};
use schema::value::platform::Platform;
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
    async fn get_upload_solution(
        &self,
        user: &User,
        platform: Platform,
        resource: &TransferResource
    ) -> Result<Upload, CloudStorageErrors> {
        let file_size = resource.size_in_bytes();
        let (chunk_size, chunk_stream_enabled) = match (resource.r#type(), platform) {
            (TransferResourceType::Folder, Platform::Web) => (Some(8 * MB), true),
            _ => 'dynamically_choose_chunk_size: {
                if file_size <= 5 * MB {
                    break 'dynamically_choose_chunk_size (Some(file_size), false);
                }

                let mut best: Option<(u64, u64)> = None;

                for leftover in 2..=9 {
                    let chunkable = file_size - leftover;

                    let min_count = ((chunkable + 5 * GB - 1) as f64) / (5f64 * GB as f64);
                    let min_count = min_count as u64;

                    let chunk_size = chunkable / min_count;

                    if file_size % chunk_size == leftover {
                        let count = chunkable / chunk_size;

                        if best.is_none() || chunk_size > best.unwrap().0 {
                            best = Some((chunk_size, count));
                        }
                    }
                }

                (best.map(|it| it.0), false)
            }
        };

        let source = resource.source();
        let upload_id = self.s3_client.create_multipart_upload(&source).await?;
        let upload_url = self
            .s3_client
            .generate_part_upload_url(&source, &upload_id, 1, self.get_download_duration())
            .await?;

        let context = UploadContext::new(
            user.id.clone(),
            upload_id,
            source.clone(),
            file_size,
            chunk_size,
            chunk_stream_enabled
        )?;

        let token = context.as_token(self.get_jwt_secret());
        let part = MultiPartUpload {
            context_token: token,
            upload_url,
            x_content_length: context.x_content_length,
            chunk_stream_enabled,
            is_last: context.is_last()
        };

        Ok(Upload::Multipart(part))
    }

    async fn get_upload_url(&self, source: &StaticResource) -> Result<String, CloudStorageErrors> {
        let duration = self.get_download_duration();
        let single_url = self.s3_client.sign_upload(source, duration).await?;
        Ok(single_url)
    }

    async fn complete_upload_part(&self, user: &User, context_token: &str) -> Result<Option<MultiPartUpload>, CloudStorageErrors> {
        let context = UploadContext::from_token(context_token, self.get_jwt_secret(), user)?;
        let chunk_stream_enabled = context.chunk_stream_enabled;
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
            context_token: next_part.as_token(self.get_jwt_secret()),
            x_content_length: next_part.x_content_length,
            chunk_stream_enabled,
            is_last: next_part.is_last()
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
