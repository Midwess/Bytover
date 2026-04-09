use crate::db::repository::abstraction::errors::RepositoryError;

impl From<redb::TransactionError> for RepositoryError {
    fn from(error: redb::TransactionError) -> Self {
        Self::DbError(format!("{error:?}"))
    }
}

impl From<redb::TableError> for RepositoryError {
    fn from(error: redb::TableError) -> Self {
        Self::DbError(format!("{error:?}"))
    }
}

impl From<redb::Error> for RepositoryError {
    fn from(error: redb::Error) -> Self {
        Self::DbError(format!("{error:?}"))
    }
}

impl From<redb::StorageError> for RepositoryError {
    fn from(error: redb::StorageError) -> Self {
        Self::DbError(format!("{error:?}"))
    }
}

impl From<redb::CommitError> for RepositoryError {
    fn from(error: redb::CommitError) -> Self {
        Self::DbError(format!("{error:?}"))
    }
}
