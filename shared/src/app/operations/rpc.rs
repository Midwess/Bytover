use std::future::Future;

use crux_core::capability::Operation;
use crux_core::Command;
use serde::{Deserialize, Serialize};

use crate::app::AppRequestBuilder;
use crate::entities::device::DeviceInfo;
use crate::entities::user::User;
use crate::errors::CoreError;

use super::{CoreOperation, CoreOperationOutput};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RpcOperation {
    GetSignInUrl(DeviceInfo),
    GetMe()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RpcOperationOutput {
    NetworkError(CoreError),
    SignInUrl(String),
    GetMe(User)
}

impl Operation for RpcOperation {
    type Output = RpcOperationOutput;
}

impl RpcOperation {
    pub fn get_me() -> AppRequestBuilder<impl Future<Output = Result<User, CoreError>>> {
        Command::request_from_shell(CoreOperation::Rpc(RpcOperation::GetMe())).map(|res| match res {
            CoreOperationOutput::Rpc(RpcOperationOutput::GetMe(user)) => Ok(user),
            CoreOperationOutput::Rpc(RpcOperationOutput::NetworkError(error)) => Err(error),
            _ => panic!("Invalid output for RpcOperation::GetMe")
        })
    }

    pub fn get_sign_in_url(device_info: DeviceInfo) -> AppRequestBuilder<impl Future<Output = Result<String, CoreError>>> {
        Command::request_from_shell(CoreOperation::Rpc(RpcOperation::GetSignInUrl(device_info))).map(|res| match res {
            CoreOperationOutput::Rpc(RpcOperationOutput::SignInUrl(url)) => Ok(url),
            CoreOperationOutput::Rpc(RpcOperationOutput::NetworkError(error)) => Err(error),
            _ => panic!("Invalid output for RpcOperation::GetSignInUrl")
        })
    }
}
