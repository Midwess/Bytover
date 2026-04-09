use crate::db::repository::abstraction::errors::RepositoryError;
use serde_wasm_bindgen::Error;

impl From<Error> for RepositoryError {
    fn from(value: Error) -> Self {
        Self::DbError(format!("{:?}", value))
    }
}

impl From<idb::Error> for RepositoryError {
    fn from(value: idb::Error) -> Self {
        Self::DbError(format!("{:?}", value))
    }
}
