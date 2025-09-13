pub mod device;
pub mod dialog;
pub mod internet;
pub mod p2p;
pub mod persistent;
pub mod rpc;
pub mod transfer;
pub mod webview;

use std::time::Duration;

use crux_core::capability::Operation;
use device::{DeviceOperation, DeviceOperationOutput};
use dialog::{DialogOperation, DialogOperationOutput};
use internet::{InternetOperation, InternetOperationOutput};
use p2p::{P2POperation, P2POperationOutput};
use persistent::{PersistentOperation, PersistentOperationOutput};
use rpc::{RpcOperation, RpcOperationOutput};
use serde::{Deserialize, Serialize};
use transfer::{TransferOperation, TransferOperationOutput};
use webview::{WebViewOperation, WebViewOperationOutput};

use crate::errors::{DeviceError, NetworkError};

use super::AppEvent;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CoreOperationOutput {
    WebView(WebViewOperationOutput),
    Device(DeviceOperationOutput),
    Rpc(RpcOperationOutput),
    Database(PersistentOperationOutput),
    Transfer(TransferOperationOutput),
    P2P(P2POperationOutput),
    Internet(InternetOperationOutput),
    InitNativeExecutor,
    Void,
    ConnectionError(NetworkError),
    DeviceError(DeviceError),
    Dialog(DialogOperationOutput),
    Delay()
}

impl Operation for CoreOperation {
    type Output = CoreOperationOutput;
}
