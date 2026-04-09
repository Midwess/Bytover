use crate::services::base::Resolve;
use crate::services::errors::Errors;
use aws_sdk_s3::error::SdkError;
use aws_sdk_s3::operation::head_object::HeadObjectError;
use aws_sdk_s3::presigning::PresigningConfig;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::types::{CompletedMultipartUpload, CompletedPart};
use aws_sdk_s3::Client;
use schema::value::static_resource::static_resource::Source;
use schema::value::static_resource::{S3Path, StaticResource};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UploadPart {
    pub part_number: i32,
    pub upload_url: String,
    // None means the client will decide
    pub x_content_length: Option<u64>
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MultipartUpload {
    pub upload_id: String,
    pub parts: Vec<UploadPart>
}

pub struct S3ClientResourceProvider;

#[derive(Clone)]
pub struct S3Client {
    pub client: Arc<Client>
}

impl S3Client {
    pub async fn get_objects(&self, path: S3Path) -> Resolve<Vec<StaticResource>> {
        let list_objects_resp =
            self.client.list_objects_v2().bucket(path.bucket()).prefix(path.prefix()).send().await?;

        let mut result: Vec<StaticResource> = vec![];
        for object in list_objects_resp.contents() {
            let prefix = object.key().unwrap().to_owned();
            // Ignoring it self
            if path.prefix().ne(&prefix) {
                result.push(StaticResource {
                    source: Some(Source::S3Path(S3Path::new(path.bucket().to_owned(), prefix)))
                })
            }
        }

        Ok(result)
    }

    pub async fn create_object(&self, path: S3Path, content: Vec<u8>) -> Resolve<()> {
        self.client
            .put_object()
            .bucket(path.bucket())
            .key(path.prefix())
            .content_length(content.len() as i64)
            .body(ByteStream::from(content))
            .send()
            .await?;

        Ok(())
    }

    pub async fn delete_object(&self, path: S3Path) -> Resolve<()> {
        self.client.delete_object().bucket(path.bucket()).key(path.prefix()).send().await?;

        Ok(())
    }

    pub async fn head_object(&self, path: S3Path) -> Resolve<()> {
        let result = self.client.head_object().bucket(path.bucket()).key(path.prefix()).send().await;

        match result {
            Ok(_) => Ok(()),
            Err(SdkError::ServiceError(e)) if matches!(e.err(), HeadObjectError::NotFound(_)) => {
                Err(Errors::S3NotFound(format!("S3 object not found: {:?}", path)))
            }
            Err(e) => Err(e.into())
        }
    }

    pub async fn download(&self, path: S3Path) -> Resolve<Option<Vec<u8>>> {
        // Get the object, handle the case where it might not exist
        let download_result = self.client.get_object().bucket(path.bucket()).key(path.prefix()).send().await;

        match download_result {
            Ok(download_url) => {
                let body = match download_url.body.collect().await {
                    Ok(body) => body,
                    Err(_) => return Ok(None)
                };
                Ok(Some(body.to_vec()))
            }
            Err(_) => Ok(None)
        }
    }

    pub async fn use_cdn(&self, s3_object: &mut StaticResource, cdn_host_url: String) -> Resolve<()> {
        let path = match &s3_object.source {
            Some(Source::S3Path(path)) => path.clone(),
            _ => return Err(Errors::BadRequest("Bad request, static resource must be a s3 path".to_owned()))
        };

        let cdn_url = format!("{cdn_host_url}/{}", path.prefix());

        s3_object.source = Some(Source::Url(cdn_url.to_owned()));

        Ok(())
    }

    pub async fn sign_download(&self, s3_object: &StaticResource, duration: Duration) -> Resolve<String> {
        let path = match &s3_object.source {
            Some(Source::S3Path(path)) => path.clone(),
            Some(Source::Url(url)) => return Ok(url.clone()),
            _ => return Err(Errors::BadRequest("Bad request, static resource must be a s3 path".to_owned()))
        };

        let presigning_config = PresigningConfig::expires_in(duration)?;
        let presigned_req = self
            .client
            .get_object()
            .bucket(path.bucket())
            .key(path.prefix())
            .response_content_disposition("attachment")
            .presigned(presigning_config)
            .await?;

        let url = presigned_req.uri().to_owned();

        Ok(url)
    }

    pub async fn sign_upload(&self, s3_object: &StaticResource, duration: Duration) -> Resolve<String> {
        let path = match &s3_object.source {
            Some(Source::S3Path(path)) => path.clone(),
            Some(Source::Url(url)) => return Ok(url.clone()),
            _ => return Err(Errors::BadRequest("Bad request, static resource must be a s3 path".to_owned()))
        };

        let presigning_config = PresigningConfig::expires_in(duration)?;
        let presigned_req = self
            .client
            .put_object()
            .bucket(path.bucket())
            .key(path.prefix())
            .presigned(presigning_config)
            .await?;

        let url = presigned_req.uri().to_owned();

        Ok(url)
    }

    pub async fn create_multipart_upload(&self, s3_object: &StaticResource) -> Resolve<String> {
        let path = match &s3_object.source {
            Some(Source::S3Path(path)) => path.clone(),
            _ => return Err(Errors::BadRequest("Bad request, static resource must be a s3 path".to_owned()))
        };

        let upload_req = self.client.create_multipart_upload().bucket(path.bucket()).key(path.prefix()).send().await?;

        let Some(upload_id) = upload_req.upload_id else {
            return Err(Errors::BadRequest("Cannot generate upload id".to_owned()));
        };

        Ok(upload_id)
    }

    pub async fn generate_part_upload_url(
        &self,
        s3_object: &StaticResource,
        upload_id: &str,
        part_number: usize,
        duration: Duration
    ) -> Resolve<String> {
        let path = match &s3_object.source {
            Some(Source::S3Path(path)) => path.clone(),
            _ => return Err(Errors::BadRequest("Bad request, static resource must be a s3 path".to_owned()))
        };

        let presigning_config = PresigningConfig::expires_in(duration)?;
        let presigned_req = self
            .client
            .upload_part()
            .bucket(path.bucket())
            .key(path.prefix())
            .upload_id(upload_id)
            .part_number(part_number as i32)
            .presigned(presigning_config.clone())
            .await?;

        Ok(presigned_req.uri().to_string())
    }

    pub async fn complete_multipart_upload(
        &self,
        s3_object: &StaticResource,
        upload_id: String,
        e_tags: Vec<String>
    ) -> Resolve<()> {
        let path = match &s3_object.source {
            Some(Source::S3Path(path)) => path.clone(),
            _ => return Err(Errors::BadRequest("Bad request, static resource must be a s3 path".to_owned()))
        };

        let completed_parts: Vec<_> = e_tags
            .into_iter()
            .enumerate()
            .map(|(i, e_tag)| CompletedPart::builder().e_tag(e_tag).part_number((i + 1) as i32).build())
            .collect();

        let completed_upload = CompletedMultipartUpload::builder().set_parts(Some(completed_parts)).build();

        self.client
            .complete_multipart_upload()
            .bucket(path.bucket())
            .key(path.prefix())
            .upload_id(upload_id)
            .multipart_upload(completed_upload)
            .send()
            .await?;

        log::info!("Completed multipart upload");
        Ok(())
    }

    pub async fn abort_incomplete_multipart_uploads(&self, path: S3Path) -> Resolve<()> {
        let list_result =
            self.client.list_multipart_uploads().bucket(path.bucket()).prefix(path.prefix()).send().await?;

        for upload in list_result.uploads() {
            if let (Some(key), Some(upload_id)) = (upload.key(), upload.upload_id()) {
                if let Err(e) = self
                    .client
                    .abort_multipart_upload()
                    .bucket(path.bucket())
                    .key(key)
                    .upload_id(upload_id)
                    .send()
                    .await
                {
                    log::error!("Failed to abort multipart upload: {:?}", e);
                }
            }
        }

        Ok(())
    }
}
