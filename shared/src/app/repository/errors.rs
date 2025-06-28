use core_services::db::repository::abstraction::errors::RepositoryError;

#[derive(Debug, thiserror::Error)]
pub enum PersistenceError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] RepositoryError),
    #[error("IO error: {0}")]
    IOError(String),
    #[error("Not found {0}")]
    NotFound(String),
}
