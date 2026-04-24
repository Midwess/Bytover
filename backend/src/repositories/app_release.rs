use crate::entities::app_release;
use core_services::db::repository::abstraction::errors::RepositoryError;

#[derive(Debug, Clone)]
pub struct StoreReleaseUpsert {
    pub platform: String,
    pub version: String,
    pub store_url: String,
    pub release_notes: Option<String>,
}

#[async_trait::async_trait]
pub trait AppReleaseRepository: Send + Sync {
    async fn upsert_store_release(&self, row: StoreReleaseUpsert) -> Result<(), RepositoryError>;

    async fn latest_for_platform(&self, platform: &str) -> Result<Option<app_release::Model>, RepositoryError>;
}
