use std::future::Future;

use crux_core::{capability::Operation, Command};
use serde::{Deserialize, Serialize};

use crate::app::{modules::environment::DeviceInfo, AppRequestBuilder};

use super::{CoreOperation, CoreOperationOutput};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RpcOperation {
    GetSignInUrl(DeviceInfo)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RpcOperationOutput {
    SignInUrl(String)
}

impl Operation for RpcOperation {
    type Output = RpcOperationOutput;
}

impl RpcOperation {
    pub fn get_sign_in_url(device_info: DeviceInfo) -> AppRequestBuilder<impl Future<Output = String>> {
        Command::request_from_shell(CoreOperation::Rpc(RpcOperation::GetSignInUrl(device_info)))
            .map(|res| {
                match res {
                    CoreOperationOutput::Rpc(RpcOperationOutput::SignInUrl(url)) => url,
                    _ => panic!("Invalid output for RpcOperation::GetSignInUrl")
                }
            })
    }
}