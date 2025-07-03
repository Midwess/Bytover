use anyhow::anyhow;
use crate::errors::NetworkError;

#[derive(Debug, thiserror::Error)]
pub enum RpcErrors {
    #[error("Auth error {0}")]
    AuthError(anyhow::Error),
    #[error("Bad request {0}")]
    BadRequest(String),
    #[error("Internal server error {0}")]
    InternalServerError(String),
}
impl From<tonic::Status> for RpcErrors {
    fn from(status: tonic::Status) -> Self {
        match status.code() {
            tonic::Code::InvalidArgument => RpcErrors::BadRequest(status.message().to_string()),
            tonic::Code::NotFound => RpcErrors::BadRequest(status.message().to_string()),
            tonic::Code::AlreadyExists => RpcErrors::BadRequest(status.message().to_string()),
            tonic::Code::FailedPrecondition => RpcErrors::BadRequest(status.message().to_string()),
            tonic::Code::OutOfRange => RpcErrors::BadRequest(status.message().to_string()),

            tonic::Code::Unauthenticated => RpcErrors::AuthError(anyhow!("{status}")),
            tonic::Code::PermissionDenied => RpcErrors::AuthError(anyhow!("{status}")),
            _ => RpcErrors::InternalServerError(status.message().to_string())
        }
    }
}
