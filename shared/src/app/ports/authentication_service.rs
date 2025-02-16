use crate::app::modules::environment::DeviceInfo;

#[derive(Debug, thiserror::Error)]
pub enum AuthenticationServerError {
    #[error("Connection error")]
    ConnectionError,
    #[error("Failed to request authorization url {:?}", .0)]
    InvalidRequest(#[from] tonic::Status),
}

#[async_trait::async_trait]
pub trait AuthenticationServer: Send + Sync {
    async fn request_signin_url(&self, device: DeviceInfo) -> Result<String, AuthenticationServerError>;
}
