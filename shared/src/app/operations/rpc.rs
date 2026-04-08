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
    Feedback { email: String, message: String },
    CreateP2PSession { alias: String, signalling_key: String, signalling_route: String },
    GetDeviceAliases,
    GenPeer { device: DeviceInfo }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RpcOperationOutput {
    GetMe(User),
    GetUserById(User),
    GenPeer(crate::entities::peer::Peer)
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

    pub fn create_p2p_session(
        alias: String,
        signalling_key: String,
        signalling_route: String
    ) -> AppRequestBuilder<impl Future<Output = Result<schema::devlog::bitbridge::P2pSession, CoreError>>> {
        Command::request_from_shell(CoreOperation::Rpc(RpcOperation::CreateP2PSession {
            alias,
            signalling_key,
            signalling_route
        }))
        .map(
            |res| match res {
                CoreOperationOutput::P2PSession(session) => Ok(session),
                CoreOperationOutput::Error(error) => Err(error),
                _ => panic!("Invalid output for RpcOperation::CreateP2PSession")
            }
        )
    }

    pub fn get_device_aliases() -> AppRequestBuilder<impl Future<Output = Result<Vec<String>, CoreError>>> {
        Command::request_from_shell(CoreOperation::Rpc(RpcOperation::GetDeviceAliases)).map(|res| match res {
            CoreOperationOutput::DeviceAliases(aliases) => Ok(aliases),
            CoreOperationOutput::Error(error) => Err(error),
            _ => panic!("Invalid output for RpcOperation::GetDeviceAliases")
        })
    }

    pub fn gen_peer(device: DeviceInfo) -> AppRequestBuilder<impl Future<Output = Result<crate::entities::peer::Peer, CoreError>>> {
        Command::request_from_shell(CoreOperation::Rpc(RpcOperation::GenPeer { device })).map(|res| match res {
            CoreOperationOutput::Rpc(RpcOperationOutput::GenPeer(peer)) => Ok(peer),
            CoreOperationOutput::Error(error) => Err(error),
            _ => panic!("Invalid output for RpcOperation::GenPeer")
        })
    }
}
