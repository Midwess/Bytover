use crate::entities::local_resource::LocalResourcePath;
use crate::protocol::public_cloud::cloud_service::CloudTransferErrors;
use crate::protocol::rpc::errors::RpcErrors;
use serde::{Deserialize, Serialize};

/// Any error defined here must has friendly message
/// because it will be displayed to the user (Display trait)
/// but it's must be detailed enough to be used for debugging (Debug trait)
#[derive(Debug, thiserror::Error, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub enum NetworkError {
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
    Network(String)
}

impl From<CloudTransferErrors> for NetworkError {
    fn from(e: CloudTransferErrors) -> Self {
        Self::Network(format!("{e}"))
    }
}

impl From<RpcErrors> for NetworkError {
    fn from(e: RpcErrors) -> Self {
        Self::Network(format!("{e}"))
    }
}

#[derive(Debug, thiserror::Error, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub enum DeviceError {
    #[error("Insufficient storage")]
    StorageInsufficient(String)
}

#[derive(Debug, thiserror::Error, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub enum InputError {
    #[error("Expected an absolute path")]
    ExpectedAnAbsolutePath(LocalResourcePath)
}
