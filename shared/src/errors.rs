use crate::app::file_system::file::LocalResourcePath;
use serde::{Deserialize, Serialize};
use uniffi::Enum;

/// Any error defined here must has friendly message
/// because it will be displayed to the user (Display trait)
/// but it's must be detailed enough to be used for debugging (Debug trait)
#[derive(Debug, thiserror::Error, Enum, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub enum NetworkError {
    // Unknown error from backend, should not happend
    #[error("Error happened, please try again")]
    InternalServerError(String),
    // The upstream has something to say
    #[error("{0}")]
    BadRequest(String),
    // Should signout in this case because user is not authenticated or session is expired
    #[error("Unauthorized")]
    Unauthorized(String),
    // Internet connection issue, ask user to check internet connection
    #[error("Please check your internet connection")]
    Network(String)
}

#[derive(Debug, thiserror::Error, Enum, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub enum DeviceError {
    #[error("Insufficient storage")]
    StorageInsufficient(String)
}

#[derive(Debug, thiserror::Error, Enum, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub enum InputError {
    #[error("Expected an absolute path")]
    ExpectedAnAbsolutePath(LocalResourcePath)
}
