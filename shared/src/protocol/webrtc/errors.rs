use crate::errors::CoreError;
use crate::repository::errors::PersistenceError;
use core_services::utils::cancellation::TaskErrors;
use core_services::utils::yield_container::YieldError;
use n0_future::task::JoinError;
use prost::{DecodeError, EncodeError};

#[derive(thiserror::Error, Debug)]
pub enum WebRtcErrors {
    #[error("Signalling client error: {0}")]
    SignallingClientError(anyhow::Error),

    #[error("Received an unsupported event from the signalling server")]
    UnSupportedEventFromSignallingServer,

    #[error("Could not send your message: {0}")]
    MessageEncodeError(#[from] EncodeError),

    #[error("Could not read an incoming message: {0}")]
    MessageDecodeError(#[from] DecodeError),

    #[error("The message channel encountered an error: {0}")]
    MessageChannelError(String),

    #[error("Could not connect you with the peer")]
    FailedToIntroducePeer,

    #[error("A session is already in progress. Please wait or end it before starting a new one.")]
    SessionAlreadyInProgress,

    #[error("Persistent storage error: {0}")]
    PersistentError(#[from] PersistenceError),

    #[error("Could not read the selected file: {0}")]
    ReadFileError(String),

    #[error("The delimiter is invalid: {0}")]
    InvalidDelimiter(String),

    #[error("Could not find the peer connection: {0}")]
    ConnectionNotFound(String),

    #[error("Connection error: {0}")]
    Connection(String),

    #[error("RTC error: {0}")]
    Rtc(String),

    #[error("Signalling error: {0}")]
    Signalling(String),

    #[error("SDP error: {0}")]
    Sdp(String),

    #[error("Transfer error: {0}")]
    Transfer(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("An unexpected system error occurred: {0}")]
    SystemError(#[from] anyhow::Error),

    #[error("Canceled")]
    Canceled(#[from] TaskErrors),

    #[error("System yield error: {0}")]
    YieldError(#[from] YieldError),

    #[error("Task panic: {0}")]
    Panic(#[from] JoinError),

    #[error("Peer error: {0}")]
    PeerError(String),

    #[error("Timeout waiting for response")]
    Timeout,

    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    #[error("Turn send error")]
    TurnSendError(anyhow::Error),
}

impl From<WebRtcErrors> for CoreError {
    fn from(err: WebRtcErrors) -> Self {
        CoreError::Network(err.to_string())
    }
}
