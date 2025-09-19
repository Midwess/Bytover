use crate::cloud_storage::storage::{CloudStorage, CloudStorageErrors, UploadContext};
use crate::entities::transfer_resource::TransferResource;
use core_services::s3::S3Client;
use core_services::token::jwt::{create_jwt_token, decode_jwt_token};
use schema::devlog::bitbridge::client_upload_request::Upload;
use schema::devlog::bitbridge::{MultiPartUpload, MultiPartUploadComplete, UploadPart};
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
    async fn get_upload_solution_for_resource(&self, resource: &TransferResource) -> Result<Upload, CloudStorageErrors> {
        let file_size = Some(resource.size_in_bytes() as usize);
        let source = resource.source();
        log::info!("Get upload solution for resource: {:?} size: {:?}", source, file_size);
        self.get_upload_solution(&source, file_size).await
    }

    async fn get_upload_solution(&self, source: &StaticResource, file_size: Option<usize>) -> Result<Upload, CloudStorageErrors> {
        let duration = self.get_upload_duration();
        let Some(file_size) = file_size else {
            let single_url = self.s3_client.sign_upload(source, duration).await?;
            return Ok(Upload::SingleUrl(single_url));
        };

        let part_size = self.get_max_part_size(file_size);
        let part_count = ((file_size + part_size - 1) / part_size) as i32 + self.extra_upload() as i32;

        let multipart = self
            .s3_client
            .generate_multipart_upload_urls(source, part_count, duration)
            .await?;

        let context = UploadContext {
            resource: source.clone(),
            upload_id: multipart.upload_id
        };

        let context_token = create_jwt_token(context, self.get_jwt_secret(), duration)?;

        let mut remaining_size = file_size as u64;
        let upload_parts = multipart
            .parts
            .into_iter()
            .enumerate()
            .map(|(index, part)| UploadPart {
                url: part.upload_url,
                x_content_length: {
                    if remaining_size <= 0 {
                        None
                    }
                    else {
                        let size = remaining_size.min(part_size as u64);
                        remaining_size -= size;
                        Some(size)
                    }
                },
            })
            .collect();

        Ok(Upload::MultiParts(MultiPartUpload {
            parts: upload_parts,
            context_token
        }))
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

    async fn complete_upload(&self, completion: &MultiPartUploadComplete) -> Result<(), CloudStorageErrors> {
        let context: UploadContext = decode_jwt_token(&completion.context_token, self.get_jwt_secret())?;

        self.s3_client
            .complete_multipart_upload(&context.resource, context.upload_id, completion.e_tags.clone())
            .await?;

        Ok(())
    }
}
