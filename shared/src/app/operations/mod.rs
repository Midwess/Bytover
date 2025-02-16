pub mod local_storage;
pub mod webview;
pub mod device;
pub mod rpc;
pub mod database;

use crux_core::{capability::{CapabilityContext, Operation}, macros::Capability, Command};
use database::{DatabaseOperation, DatabaseOperationOutput};
use device::{DeviceOperation, DeviceOperationOutput};
use local_storage::{LocalStorageOperation, LocalStorageOperationOutput};
use rpc::{RpcOperation, RpcOperationOutput};
use serde::{Deserialize, Serialize};
use webview::{WebViewOperation, WebViewOperationOutput};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CoreOperation {
    LocalStorage(LocalStorageOperation),
    WebView(WebViewOperation),
    Device(DeviceOperation),
    Rpc(RpcOperation),
    Database(DatabaseOperation),
    Void
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CoreOperationOutput {
    LocalStorage(LocalStorageOperationOutput),
    WebView(WebViewOperationOutput),
    Device(DeviceOperationOutput),
    Rpc(RpcOperationOutput),
    Database(DatabaseOperationOutput),
    Void
}

impl Operation for CoreOperation {
    type Output = CoreOperationOutput;
}
