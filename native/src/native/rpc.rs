use shared::app::operations::rpc::{RpcOperation, RpcOperationOutput};
use crate::di_container::DiContainer;

pub struct NativeRpc {}

impl NativeRpc {
    pub async fn handle(&self, effect: RpcOperation) -> RpcOperationOutput {
        match effect {
            RpcOperation::GetSignInUrl(device_info) => {
                let di_container = DiContainer::get_instance();
                let response = di_container.get_authentication_server().request_signin_url(device_info).await;
                match response {
                    Ok(url) => RpcOperationOutput::SignInUrl(url),
                    Err(e) => RpcOperationOutput::NetworkError(e.into())
                }
            }
            RpcOperation::GetMe() => {
                let di_container = DiContainer::get_instance();
                let response = di_container.get_authentication_server().get_me().await;
                match response {
                    Ok(user) => RpcOperationOutput::GetMe(user),
                    Err(e) => RpcOperationOutput::NetworkError(e.into())
                }
            }
        }
    }
}
