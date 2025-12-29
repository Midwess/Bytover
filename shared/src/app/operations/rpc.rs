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
    GetAuthenticateUrl(DeviceInfo),
    GetMe(),
    GetUserById(u64),
    Feedback {
        email: String,
        message: String,
    },
    RandomAvatar,
    CreateP2PSession,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RpcOperationOutput {
    GetMe(User),
    GetUserById(User),
}

impl Operation for RpcOperation {
    type Output = RpcOperationOutput;
}

impl RpcOperation {
    pub fn get_me() -> AppRequestBuilder<impl Future<Output = Result<User, CoreError>>> {
        Command::request_from_shell(CoreOperation::Rpc(RpcOperation::GetMe())).map(|res| match res {
            CoreOperationOutput::Rpc(RpcOperationOutput::GetMe(user)) => Ok(user),
            CoreOperationOutput::Error(error) => Err(error),
            e => panic!("Invalid output for RpcOperation::GetMe got {e:?}")
        })
    }

    pub fn get_user_by_id(user_id: u64) -> AppRequestBuilder<impl Future<Output = Result<User, CoreError>>> {
        Command::request_from_shell(CoreOperation::Rpc(RpcOperation::GetUserById(user_id))).map(|res| match res {
            CoreOperationOutput::Rpc(RpcOperationOutput::GetUserById(user)) => Ok(user),
            CoreOperationOutput::Error(error) => Err(error),
            e => panic!("Invalid output for RpcOperation::GetUserById got {e:?}")
        })
    }

    pub fn get_authenticate_url(device_info: DeviceInfo) -> AppRequestBuilder<impl Future<Output = Result<String, CoreError>>> {
        Command::request_from_shell(CoreOperation::Rpc(RpcOperation::GetAuthenticateUrl(device_info))).map(|res| match res {
            CoreOperationOutput::String(value) => Ok(value),
            CoreOperationOutput::Error(error) => Err(error),
            _ => panic!("Invalid output for RpcOperation::GetSignInUrl")
        })
    }

    pub fn feedback(email: String, message: String) -> AppRequestBuilder<impl Future<Output = Result<(), CoreError>>> {
        Command::request_from_shell(CoreOperation::Rpc(RpcOperation::Feedback { email, message })).map(|res| match res {
            CoreOperationOutput::None => Ok(()),
            CoreOperationOutput::Error(error) => Err(error),
            _ => panic!("Invalid output for RpcOperation::Feedback")
        })
    }

    pub fn random_avatar() -> AppRequestBuilder<impl Future<Output = Result<String, CoreError>>> {
        Command::request_from_shell(CoreOperation::Rpc(RpcOperation::RandomAvatar)).map(|res| match res {
            CoreOperationOutput::String(value) => Ok(value),
            CoreOperationOutput::Error(error) => Err(error),
            _ => panic!("Invalid output for RpcOperation::RandomAvatar")
        })
    }

    pub fn create_p2p_session() -> AppRequestBuilder<impl Future<Output = Result<schema::devlog::bitbridge::P2pSession, CoreError>>> {
        Command::request_from_shell(CoreOperation::Rpc(RpcOperation::CreateP2PSession)).map(|res| match res {
            CoreOperationOutput::P2PSession(session) => Ok(session),
            CoreOperationOutput::Error(error) => Err(error),
            _ => panic!("Invalid output for RpcOperation::CreateP2PSession")
        })
    }
}
