use serde::{Deserialize, Serialize};
use uniffi::Enum;

use crate::network::webrtc::connection::ConnectionWebRtcErrors;
use crate::network::webrtc::peer::PeerErrors;
use crate::network::webrtc::web_rtc::WebRtcErrors;

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

impl From<tonic::transport::Error> for NetworkError {
    fn from(err: tonic::transport::Error) -> Self {
        NetworkError::Network(err.to_string())
    }
}

impl From<tonic::Status> for NetworkError {
    fn from(status: tonic::Status) -> Self {
        match status.code() {
            tonic::Code::InvalidArgument => NetworkError::BadRequest(status.message().to_string()),
            tonic::Code::NotFound => NetworkError::BadRequest(status.message().to_string()),
            tonic::Code::AlreadyExists => NetworkError::BadRequest(status.message().to_string()),
            tonic::Code::FailedPrecondition => NetworkError::BadRequest(status.message().to_string()),
            tonic::Code::OutOfRange => NetworkError::BadRequest(status.message().to_string()),

            tonic::Code::Unknown => NetworkError::InternalServerError(status.message().to_string()),
            tonic::Code::Internal => NetworkError::InternalServerError(status.message().to_string()),
            tonic::Code::Unimplemented => NetworkError::InternalServerError(status.message().to_string()),
            tonic::Code::DataLoss => NetworkError::InternalServerError(status.message().to_string()),

            tonic::Code::Unauthenticated => NetworkError::Unauthorized(status.message().to_string()),
            tonic::Code::PermissionDenied => NetworkError::Unauthorized(status.message().to_string()),

            _ => NetworkError::Network(status.message().to_string())
        }
    }
}

#[derive(Debug, thiserror::Error, Enum, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub enum DeviceError {
    #[error("Insufficient storage")]
    StorageInsufficient(String)
}

impl From<WebRtcErrors> for NetworkError {
    fn from(err: WebRtcErrors) -> Self {
        match err {
            WebRtcErrors::ConnectionError(e) => NetworkError::Network(e.to_string()),
            WebRtcErrors::SignallingServerError(e) => NetworkError::Network(e.to_string()),
            WebRtcErrors::TransferError(e) => NetworkError::Network(e.to_string()),
            WebRtcErrors::WebRTCServerError(e) => NetworkError::Network(e.to_string())
        }
    }
}

impl From<ConnectionWebRtcErrors> for NetworkError {
    fn from(err: ConnectionWebRtcErrors) -> Self {
        NetworkError::Network(format!("{:?}", err))
    }
}

impl From<PeerErrors> for NetworkError {
    fn from(err: PeerErrors) -> Self {
        NetworkError::Network(format!("{:?}", err))
    }
}
