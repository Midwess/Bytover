use schema::devlog::app_gateway::models::Application;
use thiserror::Error;
use tonic::Status;

#[derive(Debug, Error)]
pub enum AppInfoErrors {
    #[error("Connection error: {0}")]
    ConnectionError(#[from] tonic::transport::Error),
    #[error("Server error: {0}")]
    TonicStatus(#[from] Status)
}

#[async_trait::async_trait]
pub trait AppInfoService: Send + Sync {
    async fn get_app_info(&self, app_name: String) -> Result<Option<Application>, AppInfoErrors>;
    async fn random_avatar(&self) -> Result<String, AppInfoErrors>;
}
