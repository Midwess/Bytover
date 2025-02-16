/// Any error defined here must has friendly message
/// because it will be displayed to the user (Display trait)
/// but it's must be detailed enough to be used for debugging (Debug trait)
#[derive(Debug, thiserror::Error)]
pub enum NetworkError {
    #[error("Internal server error")]
    InternalServerError,
    // The upstream has something to say
    #[error("{0}")]
    BadRequest(String)
}

#[derive(Debug, thiserror::Error)]
pub enum AuthenticationError {
    #[error("{0}")]
    NetworkError(#[from] NetworkError),
}

impl From<tonic::Status> for AuthenticationError {
    fn from(status: tonic::Status) -> Self {
        match status.code() {
            tonic::Code::Internal => AuthenticationError::NetworkError(NetworkError::InternalServerError),
            tonic::Code::Unavailable => AuthenticationError::NetworkError(NetworkError::InternalServerError),
            tonic::Code::Unimplemented => AuthenticationError::NetworkError(NetworkError::InternalServerError),
            tonic::Code::DataLoss => AuthenticationError::NetworkError(NetworkError::InternalServerError),
            _ => AuthenticationError::NetworkError(NetworkError::BadRequest(status.message().to_string()))
        }
    }
}