pub mod database;
pub mod device;
pub mod local_storage;
pub mod rpc;
pub mod transfer;
pub mod webview;
pub mod internet;

use crux_core::capability::Operation;
use database::{DatabaseOperation, DatabaseOperationOutput};
use device::{DeviceOperation, DeviceOperationOutput};
use internet::{InternetOperation, InternetOperationOutput};
use local_storage::{LocalStorageOperation, LocalStorageOperationOutput};
use rpc::{RpcOperation, RpcOperationOutput};
use serde::{Deserialize, Serialize};
use transfer::{TransferOperation, TransferOperationOutput};
use webview::{WebViewOperation, WebViewOperationOutput};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CoreOperation {
    LocalStorage(LocalStorageOperation),
    WebView(WebViewOperation),
    Device(DeviceOperation),
    Rpc(RpcOperation),
    Database(DatabaseOperation),
    Transfer(TransferOperation),
    Internet(InternetOperation),
    Render,
    Void
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CoreOperationOutput {
    LocalStorage(LocalStorageOperationOutput),
    WebView(WebViewOperationOutput),
    Device(DeviceOperationOutput),
    Rpc(RpcOperationOutput),
    Database(DatabaseOperationOutput),
    Transfer(TransferOperationOutput),
    Internet(InternetOperationOutput),
    Void
}

impl Operation for CoreOperation {
    type Output = CoreOperationOutput;
}
