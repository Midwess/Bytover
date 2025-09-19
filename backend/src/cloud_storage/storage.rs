use crate::entities::transfer_resource::TransferResource;
use core_services::token::jwt::JwtErrors;
use schema::devlog::bitbridge::client_upload_request::Upload;
use schema::devlog::bitbridge::MultiPartUploadComplete;
use schema::value::static_resource::StaticResource;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, thiserror::Error)]
pub enum CloudStorageErrors {
    #[error("Cloud storage error {0}")]
    S3Errors(#[from] core_services::services::errors::Errors),
    #[error("JWT error: {0}")]
    JwtError(#[from] JwtErrors),
    #[error("Invalid upload context")]
    InvalidUploadContext
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UploadContext {
    pub upload_id: String,
    pub resource: StaticResource
}

#[async_trait::async_trait]
pub trait CloudStorage: Send + Sync {
    async fn get_upload_solution_for_resource(&self, resource: &TransferResource) -> Result<Upload, CloudStorageErrors>;
    async fn get_upload_solution(&self, source: &StaticResource, file_size: Option<usize>) -> Result<Upload, CloudStorageErrors>;
    async fn generate_download_url(&self, source: &StaticResource) -> Result<String, CloudStorageErrors>;
    async fn complete_upload(&self, completion: &MultiPartUploadComplete) -> Result<(), CloudStorageErrors>;

    fn get_max_part_size(&self) -> usize {
        5 * 1024 * 1024 * 1024 - 1
    }

    // The number of extra url that we want to buffer for the client
    // in case they need more than usual.
    fn extra_upload(&self) -> usize {
        1
    }

    fn get_upload_duration(&self) -> Duration {
        Duration::from_secs(60 * 60 * 24 * 3)
    }

    fn get_download_duration(&self) -> Duration {
        Duration::from_secs(60 * 60 * 24 * 7)
    }

    fn get_jwt_secret(&self) -> &str {
        "default_jwt_secret_change_in_production"
    }
}
