pub type Resolve<T> = Result<T, RepositoryError>;
#[derive(Debug, thiserror::Error)]
pub enum RepositoryError {
    #[error("Not found {0} {1}")]
    NotFound(String, String),
    #[error("Conflict {0} {1}")]
    Conflict(String, String),
    #[error("Error on db side {0}")]
    DbError(String),
    #[cfg(feature = "db-red")]
    #[error("Error on db side {0}")]
    BincodeError(#[from] bincode::Error),
    #[error("Json serialization error {0:?}")]
    JsonSerializationError(#[from] serde_json::Error)
}

#[cfg(feature = "grpc-server")]
impl From<RepositoryError> for tonic::Status {
    fn from(value: RepositoryError) -> Self {
        match value {
            RepositoryError::NotFound(not_found, _) => tonic::Status::not_found(not_found.to_string()),
            RepositoryError::Conflict(conflict, _) => tonic::Status::already_exists(conflict.to_string()),
            RepositoryError::DbError(db_error) => tonic::Status::internal(db_error.to_string()),
            #[cfg(feature = "db-red")]
            RepositoryError::BincodeError(bincode_error) => tonic::Status::internal(bincode_error.to_string()),
            RepositoryError::JsonSerializationError(json_serialization_error) => {
                tonic::Status::internal(json_serialization_error.to_string())
            }
        }
    }
}
