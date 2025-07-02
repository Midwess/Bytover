use crate::app::repository::errors::PersistenceError;
use crate::errors::NetworkError;
use matchbox_protocol::PeerId;
use matchbox_socket::ChannelError;
use prost::{DecodeError, EncodeError};

#[derive(thiserror::Error, Debug)]
pub enum WebRtcErrors {
    #[error("Signalling client error: {0}")]
    SignallingClientError(anyhow::Error),
    #[error("Unsupported event from signalling server")]
    UnSupportedEventFromSignallingServer,
    #[error("Channel error")]
    ChannelErrors(#[from] ChannelError),
    #[error("Message encode error")]
    MessageEncodeError(#[from] EncodeError),
    #[error("Message decode error")]
    MessageDecodeError(#[from] DecodeError),
    #[error("Message channel error")]
    MessageChannelError(String),
    #[error("Failed to introduce peer")]
    FailedToIntroducePeer,
    #[error("Session already in-progress")]
    SessionAlreadyInProgress,
    #[error("Persistent error {0}")]
    PersistentError(#[from] PersistenceError),
    #[error("Read file error {0}")]
    ReadFileError(String),
    #[error("Invalid delimiter")]
    InvalidDelimiter(String),
    #[error("Peer connection not found {0}")]
    ConnectionNotFound(PeerId),
    #[error("System error")]
    SystemError(#[from] anyhow::Error)
}

impl From<WebRtcErrors> for matchbox_socket::SignalingError {
    fn from(val: WebRtcErrors) -> Self {
        matchbox_socket::SignalingError::UserImplementationError(format!("{val:?}"))
    }
}

impl From<WebRtcErrors> for NetworkError {
    fn from(err: WebRtcErrors) -> Self {
        NetworkError::Network(format!("{err:?}"))
    }
}
