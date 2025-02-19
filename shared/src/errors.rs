use serde::{Deserialize, Serialize};
use uniffi::Enum;

/// Any error defined here must has friendly message
/// because it will be displayed to the user (Display trait)
/// but it's must be detailed enough to be used for debugging (Debug trait)
#[derive(Debug, thiserror::Error, Enum, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub enum NetworkError {
    #[error("Internal server error")]
    InternalServerError(String),
    // The upstream has something to say
    #[error("{0}")]
    BadRequest(String),
    #[error("Unauthorized: {0}")]
    Unauthorized(String),
    #[error("Network issue")]
    Network(String)
}

impl From<tonic::transport::Error> for NetworkError {
    fn from(err: tonic::transport::Error) -> Self {
        NetworkError::Network(err.to_string())
    }
}

impl From<tonic::Status> for NetworkError {
    fn from(status: tonic::Status) -> Self {
        match status.code() {
            tonic::Code::InvalidArgument => NetworkError::BadRequest(status.message().to_string()),
            tonic::Code::NotFound => NetworkError::BadRequest(status.message().to_string()),
            tonic::Code::AlreadyExists => NetworkError::BadRequest(status.message().to_string()),
            tonic::Code::FailedPrecondition => NetworkError::BadRequest(status.message().to_string()),
            tonic::Code::OutOfRange => NetworkError::BadRequest(status.message().to_string()),

            tonic::Code::Unknown => NetworkError::InternalServerError(status.message().to_string()),
            tonic::Code::Internal => NetworkError::InternalServerError(status.message().to_string()),
            tonic::Code::Unimplemented => NetworkError::InternalServerError(status.message().to_string()),
            tonic::Code::DataLoss => NetworkError::InternalServerError(status.message().to_string()),

            tonic::Code::Unauthenticated => NetworkError::Unauthorized(status.message().to_string()),
            tonic::Code::PermissionDenied => NetworkError::Unauthorized(status.message().to_string()),

            _ => NetworkError::Network(status.message().to_string())
        }
    }
}
