use std::future::Future;

use crux_core::capability::Operation;
use crux_core::Command;
use serde::{Deserialize, Serialize};

use crate::app::AppRequestBuilder;
use crate::entities::capabilities::UserCapabilities;
use crate::entities::device::DeviceInfo;
use crate::entities::user::User;
use crate::errors::CoreError;
use crate::protocol::rpc::cloud_server::SubmitStoreKitResult;

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
    CreateP2PSession {
        alias: String,
        signalling_key: String,
        signalling_route: String,
    },
    GetDeviceAliases,
    GenAlias,
    GenPeer {
        device: DeviceInfo,
    },
    GetCapabilities,
    ReportP2PBytesUsed {
        delta: u64,
    },
    SubmitStoreKitTransaction {
        transaction_id: String,
        product_id: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RpcOperationOutput {
    GetMe { user: User, device_unique_key: String },
    GetUserById(User),
    GenPeer(crate::entities::peer::Peer),
    GetCapabilities(UserCapabilities),
    SubmitStoreKit(SubmitStoreKitResult),
}

impl Operation for RpcOperation {
    type Output = RpcOperationOutput;
}

impl RpcOperation {
    pub fn get_me() -> AppRequestBuilder<impl Future<Output = Result<(User, String), CoreError>>> {
        Command::request_from_shell(CoreOperation::Rpc(RpcOperation::GetMe())).map(|res| match res {
            CoreOperationOutput::Rpc(RpcOperationOutput::GetMe { user, device_unique_key }) => Ok((user, device_unique_key)),
            CoreOperationOutput::Error(error) => Err(error),
            e => panic!("Invalid output for RpcOperation::GetMe got {e:?}"),
        })
    }

    pub fn get_user_by_id(user_id: u64) -> AppRequestBuilder<impl Future<Output = Result<User, CoreError>>> {
        Command::request_from_shell(CoreOperation::Rpc(RpcOperation::GetUserById(user_id))).map(|res| match res {
            CoreOperationOutput::Rpc(RpcOperationOutput::GetUserById(user)) => Ok(user),
            CoreOperationOutput::Error(error) => Err(error),
            e => panic!("Invalid output for RpcOperation::GetUserById got {e:?}"),
        })
    }

    pub fn get_authenticate_url(device_info: DeviceInfo) -> AppRequestBuilder<impl Future<Output = Result<String, CoreError>>> {
        Command::request_from_shell(CoreOperation::Rpc(RpcOperation::GetAuthenticateUrl(device_info))).map(|res| match res {
            CoreOperationOutput::String(value) => Ok(value),
            CoreOperationOutput::Error(error) => Err(error),
            _ => panic!("Invalid output for RpcOperation::GetSignInUrl"),
        })
    }

    pub fn feedback(email: String, message: String) -> AppRequestBuilder<impl Future<Output = Result<(), CoreError>>> {
        Command::request_from_shell(CoreOperation::Rpc(RpcOperation::Feedback { email, message })).map(|res| match res {
            CoreOperationOutput::None => Ok(()),
            CoreOperationOutput::Error(error) => Err(error),
            _ => panic!("Invalid output for RpcOperation::Feedback"),
        })
    }

    pub fn create_p2p_session(
        alias: String,
        signalling_key: String,
        signalling_route: String,
    ) -> AppRequestBuilder<impl Future<Output = Result<schema::devlog::bitbridge::P2pSession, CoreError>>> {
        Command::request_from_shell(CoreOperation::Rpc(RpcOperation::CreateP2PSession {
            alias,
            signalling_key,
            signalling_route,
        }))
        .map(|res| match res {
            CoreOperationOutput::P2PSession(session) => Ok(session),
            CoreOperationOutput::Error(error) => Err(error),
            _ => panic!("Invalid output for RpcOperation::CreateP2PSession"),
        })
    }

    pub fn get_device_aliases() -> AppRequestBuilder<impl Future<Output = Result<Vec<String>, CoreError>>> {
        Command::request_from_shell(CoreOperation::Rpc(RpcOperation::GetDeviceAliases)).map(|res| match res {
            CoreOperationOutput::DeviceAliases(aliases) => Ok(aliases),
            CoreOperationOutput::Error(error) => Err(error),
            _ => panic!("Invalid output for RpcOperation::GetDeviceAliases"),
        })
    }

    pub fn gen_alias() -> AppRequestBuilder<impl Future<Output = Result<String, CoreError>>> {
        Command::request_from_shell(CoreOperation::Rpc(RpcOperation::GenAlias)).map(|res| match res {
            CoreOperationOutput::String(alias) => Ok(alias),
            CoreOperationOutput::Error(error) => Err(error),
            _ => panic!("Invalid output for RpcOperation::GenAlias"),
        })
    }

    pub fn gen_peer(device: DeviceInfo) -> AppRequestBuilder<impl Future<Output = Result<crate::entities::peer::Peer, CoreError>>> {
        Command::request_from_shell(CoreOperation::Rpc(RpcOperation::GenPeer { device })).map(|res| match res {
            CoreOperationOutput::Rpc(RpcOperationOutput::GenPeer(peer)) => Ok(peer),
            CoreOperationOutput::Error(error) => Err(error),
            _ => panic!("Invalid output for RpcOperation::GenPeer"),
        })
    }

    pub fn get_capabilities() -> AppRequestBuilder<impl Future<Output = Result<UserCapabilities, CoreError>>> {
        Command::request_from_shell(CoreOperation::Rpc(RpcOperation::GetCapabilities)).map(|res| match res {
            CoreOperationOutput::Rpc(RpcOperationOutput::GetCapabilities(caps)) => Ok(caps),
            CoreOperationOutput::Error(error) => Err(error),
            e => panic!("Invalid output for RpcOperation::GetCapabilities got {e:?}"),
        })
    }

    pub fn report_p2p_bytes_used(
        delta: u64,
    ) -> AppRequestBuilder<impl Future<Output = Result<UserCapabilities, CoreError>>> {
        Command::request_from_shell(CoreOperation::Rpc(RpcOperation::ReportP2PBytesUsed { delta })).map(|res| match res {
            CoreOperationOutput::Rpc(RpcOperationOutput::GetCapabilities(caps)) => Ok(caps),
            CoreOperationOutput::Error(error) => Err(error),
            e => panic!("Invalid output for RpcOperation::ReportP2PBytesUsed got {e:?}"),
        })
    }

    pub fn submit_storekit_transaction(
        transaction_id: String,
        product_id: String,
    ) -> AppRequestBuilder<impl Future<Output = Result<SubmitStoreKitResult, CoreError>>> {
        Command::request_from_shell(CoreOperation::Rpc(RpcOperation::SubmitStoreKitTransaction {
            transaction_id,
            product_id,
        }))
        .map(|res| match res {
            CoreOperationOutput::Rpc(RpcOperationOutput::SubmitStoreKit(result)) => Ok(result),
            CoreOperationOutput::Error(error) => Err(error),
            e => panic!("Invalid output for RpcOperation::SubmitStoreKitTransaction got {e:?}"),
        })
    }
}
