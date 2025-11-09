use crate::protocol::public_cloud::cloud_service::CloudTransferErrors;
use crate::protocol::rpc::errors::RpcErrors;
use serde::{Deserialize, Serialize};

/// Any error defined here must has friendly message
/// because it will be displayed to the user (Display trait)
/// but it's must be detailed enough to be used for debugging (Debug trait)
#[derive(Debug, thiserror::Error, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub enum CoreError {
    /// The upstream has something to say
    #[error("The request could not be processed.")]
    BadRequest(String),

    /// User is not authenticated or session expired
    #[error("Your are not authorized to perform this action.")]
    Unauthorized(String),

    /// Internet connection issue
    #[error("{0}")]
    Network(String),

    #[error("Browser error {0}")]
    BrowserError(String),

    #[error("")]
    ParsingError(String),

    #[error("")]
    NotImplemented(String)
}

impl From<CloudTransferErrors> for CoreError {
    fn from(e: CloudTransferErrors) -> Self {
        Self::Network(format!("{e}"))
    }
}

impl From<RpcErrors> for CoreError {
    fn from(e: RpcErrors) -> Self {
        Self::Network(format!("{e}"))
    }
}

impl From<crate::repository::errors::PersistenceError> for CoreError {
    fn from(e: crate::repository::errors::PersistenceError) -> Self {
        Self::BrowserError(format!("{e}"))
    }
}

impl From<core_services::db::repository::abstraction::errors::RepositoryError> for CoreError {
    fn from(e: core_services::db::repository::abstraction::errors::RepositoryError) -> Self {
        Self::BrowserError(format!("{e}"))
    }
}
