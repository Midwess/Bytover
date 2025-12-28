use crate::errors::CoreError;
use crate::repository::errors::PersistenceError;
use core_services::utils::cancellation::TaskErrors;
use core_services::utils::yield_container::YieldError;
use matchbox_protocol::PeerId;
use matchbox_socket::ChannelError;
use n0_future::task::JoinError;
use prost::{DecodeError, EncodeError};
use crate::protocol::webrtc::fec::FecError;

#[derive(thiserror::Error, Debug)]
pub enum WebRtcErrors {
    #[error("Something went wrong with the signalling client")]
    SignallingClientError(anyhow::Error),

    #[error("Received an unsupported event from the signalling server")]
    UnSupportedEventFromSignallingServer,

    #[error("A communication channel failed")]
    ChannelErrors(#[from] ChannelError),

    #[error("Could not send your message (encoding failed)")]
    MessageEncodeError(#[from] EncodeError),

    #[error("Could not read an incoming message (decoding failed)")]
    MessageDecodeError(#[from] DecodeError),

    #[error("The message channel encountered an error")]
    MessageChannelError(String),

    #[error("Could not connect you with the peer")]
    FailedToIntroducePeer,

    #[error("A session is already in progress. Please wait or end it before starting a new one.")]
    SessionAlreadyInProgress,

    #[error("A persistent storage error occurred")]
    PersistentError(#[from] PersistenceError),

    #[error("The selected file is not valid")]
    ReadFileError(String),

    #[error("The delimiter you provided is invalid")]
    InvalidDelimiter(String),

    #[error("Could not find the peer connection")]
    ConnectionNotFound(PeerId),

    #[error("An unexpected system error occurred")]
    SystemError(#[from] anyhow::Error),

    #[error("Canceled")]
    Canceled(#[from] TaskErrors),

    #[error("System error, yield error")]
    YieldError(#[from] YieldError),
    #[error("uuid parse error: {0}")]
    Uuid(#[from] uuid::Error),

    #[error("Data corrupted")]
    FecError(#[from] FecError),

    #[error("Panic")]
    Panic(#[from] JoinError),

    #[error("Peer error: {0}")]
    PeerError(String),

    #[error("Timeout waiting for response")]
    Timeout,

    #[error("Invalid response: {0}")]
    InvalidResponse(String),
}

impl From<WebRtcErrors> for matchbox_socket::SignalingError {
    fn from(val: WebRtcErrors) -> Self {
        matchbox_socket::SignalingError::UserImplementationError(format!("{val:?}"))
    }
}

impl From<WebRtcErrors> for CoreError {
    fn from(err: WebRtcErrors) -> Self {
        CoreError::Network(format!("WebRtc {err:?}"))
    }
}
