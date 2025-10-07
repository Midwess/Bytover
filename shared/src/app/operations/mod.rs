pub mod device;
pub mod dialog;
pub mod internet;
pub mod p2p;
pub mod persistent;
pub mod rpc;
pub mod transfer;
pub mod webview;

use std::fmt::Debug;
use std::time::Duration;

use crate::app::operations::device::GeoLocation;
use crate::entities::device::DeviceInfo;
use crate::entities::finding_scope::FindingScope;
use crate::entities::local_resource::{LocalResource, LocalResourcePath, ResourceType};
use crate::entities::peer::Peer;
use crate::entities::session::Session;
use crate::entities::token::Token;
use crate::entities::transfer_session::TransferSession;
use crate::entities::user::User;
use crate::errors::{DeviceError, NetworkError};
use crux_core::capability::Operation;
use derive_more::with_trait::TryInto;
use derive_more::{From, TryFrom};
use device::DeviceOperation;
use dialog::DialogOperation;
use internet::InternetOperation;
use p2p::{P2POperation, P2POperationOutput};
use persistent::{PersistentOperation, PersistentOperationOutput};
use rpc::{RpcOperation, RpcOperationOutput};
use serde::{Deserialize, Serialize};
use transfer::{TransferOperation, TransferOperationOutput};
use webview::WebViewOperation;

use super::AppEvent;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, From)]
pub enum CoreOperation {
    WebView(WebViewOperation),
    Device(DeviceOperation),
    Rpc(RpcOperation),
    Persistent(PersistentOperation),
    Transfer(TransferOperation),
    P2P(P2POperation),
    Internet(InternetOperation),
    Render,
    InitNativeExecutor,
    Notified(AppEvent),
    Dialog(DialogOperation),
    Delay(Duration)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, From, TryFrom, TryInto)]
pub enum CoreOperationOutput {
    Rpc(RpcOperationOutput),
    Persistent(PersistentOperationOutput),
    Transfer(TransferOperationOutput),
    P2P(P2POperationOutput),

    // ==== Entities ====
    ResourceType(ResourceType),
    LocalResourcePath(LocalResourcePath),
    Token(Token),
    User(User),
    Peer(Peer),
    AuthSession(Session),
    TransferSession(TransferSession),
    GeoLocation(GeoLocation),
    DeviceInfo(DeviceInfo),
    ThumbnailPng(Vec<u8>),
    FindingScopes(Vec<FindingScope>),
    Bool(bool),
    TransferSessions(Vec<TransferSession>),
    LocalResources(Vec<LocalResource>),

    Error(CoreError),

    None, // or Void

    // ====== Deprecated ======
    ConnectionError(NetworkError), // Deprecated, use Error instead
    DeviceError(DeviceError),      // Deprecated, use Error instead
    Void                           // Deprecated, use None instead
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, From, TryFrom, TryInto)]
pub enum CoreError {
    ConnectionError(NetworkError),
    DeviceError(DeviceError),
    ParsingError(String)
}

impl Operation for CoreOperation {
    type Output = CoreOperationOutput;
}

impl CoreOperationOutput {
    /// Convert into Option<T> — returns None if output is `None` or `Void`
    pub fn option<T>(self) -> Option<T>
    where
        T: TryFrom<Self> + Debug,
        <T as TryFrom<CoreOperationOutput>>::Error: Debug
    {
        T::try_from(self).ok()
    }

    /// Convert into Result<T, E> —
    /// - `Ok(T)` if it’s not an error variant (`ConnectionError` or `DeviceError`)
    /// - `Err(E)` if matches the expected error type
    pub fn result<T>(self) -> Result<T, CoreError>
    where
        T: TryFrom<Self> + Debug,
        <T as TryFrom<CoreOperationOutput>>::Error: Debug
    {
        match self {
            CoreOperationOutput::Error(e) => Err(e),
            _ => Ok(T::try_from(self).map_err(|e| CoreError::ParsingError(format!("{:?}", e)))?)
        }
    }
}
