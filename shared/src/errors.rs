use crate::entities::local_resource::LocalResourcePath;
use crate::protocol::public_cloud::cloud_service::CloudTransferErrors;
use crate::protocol::rpc::errors::RpcErrors;
use serde::{Deserialize, Serialize};

/// Any error defined here must has friendly message
/// because it will be displayed to the user (Display trait)
/// but it's must be detailed enough to be used for debugging (Debug trait)
#[derive(Debug, thiserror::Error, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub enum CoreError {
    /// Unknown error from backend, should not happen
    #[error("Something went wrong, please try again.")]
    InternalServerError(String),

    /// The upstream has something to say
    #[error("The request could not be processed.")]
    BadRequest(String),

    /// User is not authenticated or session expired
    #[error("Your are not authorized to perform this action.")]
    Unauthorized(String),

    /// Internet connection issue
    #[error("Check your internet connection and try again.")]
    Network(String),

    #[error("Insufficient storage")]
    StorageInsufficient(String),

    #[error("Expected an absolute path")]
    ExpectedAnAbsolutePath(LocalResourcePath),

    #[error("{0}")]
    ParsingError(String),
    // The request dropped without a response.
    // then the CoreRequest will automatically response this message to prevent core hang forever.
    #[error("No response from the executor.")]
    NoResponse
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
