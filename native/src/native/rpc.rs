use shared::app::operations::rpc::{RpcOperation, RpcOperationOutput};
use shared::rpc::auth_server::AuthServer;
use tonic::transport::Channel;

pub struct NativeRpc {
    pub auth_server: AuthServer<Channel>
}

impl NativeRpc {
    pub async fn handle(&self, effect: RpcOperation) -> RpcOperationOutput {
        match effect {
            RpcOperation::GetSignInUrl(device_info) => {
                let response = self.auth_server.request_signin_url(device_info).await;
                match response {
                    Ok(url) => RpcOperationOutput::SignInUrl(url),
                    Err(e) => RpcOperationOutput::NetworkError(e.into())
                }
            }
            RpcOperation::GetMe() => {
                let response = self.auth_server.get_me().await;
                match response {
                    Ok(user) => RpcOperationOutput::GetMe(user),
                    Err(e) => RpcOperationOutput::NetworkError(e.into())
                }
            }
        }
    }
}
