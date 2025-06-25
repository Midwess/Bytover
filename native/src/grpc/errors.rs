use shared::errors::NetworkError;

#[derive(thiserror::Error, Debug)]
pub enum NativeGrpcErrors {
    #[error("Network error")]
    Network(#[from] tonic::transport::Error),
    #[error("Bad request: {0}")]
    BadRequest(String),
    #[error("Unauthorized: {0}")]
    Unauthorized(String),
    #[error("{0}")]
    Connection(#[from] NetworkError)
}

impl From<tonic::Status> for NativeGrpcErrors {
    fn from(status: tonic::Status) -> Self {
        match status.code() {
            tonic::Code::InvalidArgument => NativeGrpcErrors::BadRequest(status.message().to_string()),
            tonic::Code::NotFound => NativeGrpcErrors::BadRequest(status.message().to_string()),
            tonic::Code::AlreadyExists => NativeGrpcErrors::BadRequest(status.message().to_string()),
            tonic::Code::FailedPrecondition => NativeGrpcErrors::BadRequest(status.message().to_string()),
            tonic::Code::OutOfRange => NativeGrpcErrors::BadRequest(status.message().to_string()),

            tonic::Code::Unauthenticated => NativeGrpcErrors::Unauthorized(status.message().to_string()),
            tonic::Code::PermissionDenied => NativeGrpcErrors::Unauthorized(status.message().to_string()),

            _ => NativeGrpcErrors::Connection(NetworkError::InternalServerError(status.message().to_string()))
        }
    }
}
