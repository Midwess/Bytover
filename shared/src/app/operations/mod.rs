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
use crate::errors::CoreError;
use crux_core::capability::Operation;
use derive_more::{From, TryFrom, TryInto};
use device::DeviceOperation;
use dialog::DialogOperation;
use internet::InternetOperation;
use p2p::{P2POperation, P2POperationOutput};
use persistent::PersistentOperation;
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
    Transfer(TransferOperationOutput),
    P2P(P2POperationOutput),

    // ==== Entities ====
    String(String),
    ResourceType(ResourceType),
    LocalResourcePath(LocalResourcePath),
    Token(Token),
    User(User),
    Peer(Peer),
    AuthSession(Session),
    TransferSession(TransferSession),
    P2PSession(schema::devlog::bitbridge::P2pSession),
    GeoLocation(GeoLocation),
    DeviceInfo(DeviceInfo),
    ThumbnailPng(Vec<u8>),
    FindingScopes(Vec<FindingScope>),
    Bool(bool),
    TransferSessions(Vec<TransferSession>),
    LocalResources(Vec<LocalResource>),
    LocalResource(LocalResource),
    ResourcePathMap(std::collections::HashMap<u64, LocalResourcePath>),

    Error(CoreError),

    None // or Void
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

    pub fn result_option<T>(self) -> Result<Option<T>, CoreError>
    where
        T: TryFrom<Self> + Debug,
        <T as TryFrom<CoreOperationOutput>>::Error: Debug
    {
        match self {
            CoreOperationOutput::Error(e) => Err(e),
            CoreOperationOutput::None => Ok(None),
            _ => Ok(Some(
                T::try_from(self).map_err(|e| CoreError::ParsingError(format!("{:?}", e)))?
            ))
        }
    }

    pub fn empty(&self) {}
}

impl<T> From<Option<T>> for CoreOperationOutput
where
    T: Into<CoreOperationOutput>
{
    fn from(option: Option<T>) -> Self {
        option.map(Into::into).unwrap_or(CoreOperationOutput::None)
    }
}

impl<T> From<Result<T, CoreError>> for CoreOperationOutput
where
    T: Into<CoreOperationOutput>
{
    fn from(result: Result<T, CoreError>) -> Self {
        match result {
            Ok(output) => output.into(),
            Err(error) => error.into()
        }
    }
}
