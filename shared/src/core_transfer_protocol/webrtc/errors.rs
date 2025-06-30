use matchbox_socket::ChannelError;
use prost::{DecodeError, EncodeError};
use schema::devlog::rpc_signalling::server::ParseIceCandidateError;
use crate::app::repository::errors::PersistenceError;

#[derive(thiserror::Error, Debug)]
pub enum WebRtcErrors {
   #[error("Signalling client error: {0}")]
   SignallingClientError(anyhow::Error),
   #[error("Ice invalid format")]
   IceInvalidFormat(ParseIceCandidateError),
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
}

impl From<ParseIceCandidateError> for WebRtcErrors {
   fn from(e: ParseIceCandidateError) -> Self {
      Self::IceInvalidFormat(e)
   }
}

impl Into<matchbox_socket::SignalingError> for WebRtcErrors {
   fn into(self) -> matchbox_socket::SignalingError {
      matchbox_socket::SignalingError::UserImplementationError(format!("{self:?}"))
   }
}
