use crate::entities::transfer_resource::TransferResource;
use core_services::token::jwt;
use core_services::token::jwt::{create_jwt_token, JwtErrors};
use schema::devlog::auth_gateway::models::user::UserId;
use schema::devlog::auth_gateway::models::User;
use schema::devlog::bitbridge::client_upload_request::Upload;
use schema::devlog::bitbridge::{MultiPartUpload, MultiPartUploadComplete};
use schema::value::platform::Platform;
use schema::value::static_resource::static_resource::Source;
use schema::value::static_resource::StaticResource;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, thiserror::Error)]
pub enum CloudStorageErrors {
    #[error("S3 error {0:?}")]
    S3Errors(#[from] core_services::services::errors::Errors),
    #[error("JWT error: {0}")]
    JwtError(#[from] JwtErrors),
    #[error("Invalid upload context")]
    InvalidUploadContext,
    #[error("Max upload part reached")]
    MaxUploadPartReached
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UploadContext {
    pub user_id: UserId,
    pub upload_id: String,
    pub part_number: usize,
    pub max_allowed_parts: usize,
    pub resource: StaticResource,
    pub x_content_length: u32,
    pub chunk_stream_enabled: bool
}

pub const MB: u64 = 1024 * 1024;
pub const GB: u64 = 1024 * 1024 * 1024;

impl UploadContext {
    pub fn new(
        user_id: UserId,
        upload_id: String,
        resource: StaticResource,
        resource_size: u64,
        chunk_size: Option<u64>,
        chunk_stream_enabled: bool
    ) -> Result<UploadContext, CloudStorageErrors> {
        let chunk_size = chunk_size.unwrap_or(5 * GB);
        let max_allowed_parts = match &resource.source {
            Some(Source::S3Path(name)) => resource_size.div_ceil(chunk_size) as usize + 1,
            _ => {
                log::error!("Invalid resource type: {:?}", resource);
                return Err(CloudStorageErrors::InvalidUploadContext)
            }
        };

        Ok(Self {
            chunk_stream_enabled,
            max_allowed_parts,
            x_content_length: chunk_size as u32,
            part_number: 1,
            upload_id,
            user_id,
            resource
        })
    }

    pub fn next(&self) -> Result<UploadContext, CloudStorageErrors> {
        let mut context = self.clone();
        context.part_number += 1;
        if context.part_number > context.max_allowed_parts {
            return Err(CloudStorageErrors::InvalidUploadContext);
        }

        Ok(context)
    }

    pub fn as_token(&self, secret: &str) -> String {
        create_jwt_token(self, secret, Duration::from_secs(60 * 60 * 7)).unwrap()
    }

    pub fn from_token(token: &str, secret: &str, user: &User) -> Result<UploadContext, CloudStorageErrors> {
        let context: UploadContext = jwt::decode_jwt_token(token, secret)?;
        if context.user_id != user.id {
            return Err(CloudStorageErrors::InvalidUploadContext);
        }

        Ok(context)
    }
}

#[async_trait::async_trait]
pub trait CloudStorage: Send + Sync {
    async fn get_upload_solution(
        &self,
        user: &User,
        platform: Platform,
        resource: &TransferResource
    ) -> Result<Upload, CloudStorageErrors>;
    async fn get_upload_url(&self, source: &StaticResource) -> Result<String, CloudStorageErrors>;
    async fn complete_upload_part(&self, user: &User, context_token: &str) -> Result<Option<MultiPartUpload>, CloudStorageErrors>;
    async fn complete_upload(&self, user: &User, completion: &MultiPartUploadComplete) -> Result<(), CloudStorageErrors>;
    async fn generate_download_url(&self, source: &StaticResource) -> Result<String, CloudStorageErrors>;
    fn get_download_duration(&self) -> Duration {
        Duration::from_secs(60 * 60 * 24 * 7)
    }
    fn get_jwt_secret(&self) -> &str {
        "default_jwt_secret_change_in_production"
    }
}
