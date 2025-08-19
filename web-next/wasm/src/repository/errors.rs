use shared::app::repository::errors::PersistenceError;
use crate::file_api::cache::BrowserCacheErrors;

impl From<BrowserCacheErrors> for PersistenceError {
    fn from(value: BrowserCacheErrors) -> Self {
        Self::IOError(value.to_string())
    }
}
