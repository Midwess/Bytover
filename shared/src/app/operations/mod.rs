pub mod database;
pub mod device;
pub mod dialog;
pub mod internet;
pub mod local_storage;
pub mod p2p;
pub mod rpc;
pub mod transfer;
pub mod webview;

use crux_core::capability::Operation;
use database::{DatabaseOperation, DatabaseOperationOutput};
use device::{DeviceOperation, DeviceOperationOutput};
use dialog::{DialogOperation, DialogOperationOutput};
use internet::{InternetOperation, InternetOperationOutput};
use local_storage::{LocalStorageOperation, LocalStorageOperationOutput};
use p2p::{P2POperation, P2POperationOutput};
use rpc::{RpcOperation, RpcOperationOutput};
use serde::{Deserialize, Serialize};
use transfer::{TransferOperation, TransferOperationOutput};
use uniffi::Enum;
use webview::{WebViewOperation, WebViewOperationOutput};

use crate::errors::{DeviceError, NetworkError};

use super::AppEvent;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CoreOperation {
    LocalStorage(LocalStorageOperation),
    WebView(WebViewOperation),
    Device(DeviceOperation),
    Rpc(RpcOperation),
    Database(DatabaseOperation),
    Transfer(TransferOperation),
    P2P(P2POperation),
    Internet(InternetOperation),
    Render,
    InitNativeExecutor,
    Void,
    Notified(AppEvent),
    Dialog(DialogOperation)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Enum)]
pub enum CoreOperationOutput {
    LocalStorage(LocalStorageOperationOutput),
    WebView(WebViewOperationOutput),
    Device(DeviceOperationOutput),
    Rpc(RpcOperationOutput),
    Database(DatabaseOperationOutput),
    Transfer(TransferOperationOutput),
    P2P(P2POperationOutput),
    Internet(InternetOperationOutput),
    InitNativeExecutor,
    Void,
    ConnectionError(NetworkError),
    DeviceError(DeviceError),
    Dialog(DialogOperationOutput)
}

impl Operation for CoreOperation {
    type Output = CoreOperationOutput;
}
