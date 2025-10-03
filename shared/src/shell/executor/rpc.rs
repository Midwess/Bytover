use crate::app::operations::rpc::{RpcOperation, RpcOperationOutput};
use crate::protocol::rpc::auth_server::AuthServer;
use core_services::utils::maybe::MaybeSend;

#[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
#[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
pub trait NativeRpc<T>: Send + Sync
where
    T: Clone,
    T: MaybeSend + Sync,
    T: tonic::client::GrpcService<tonic::body::Body>,
    T::Future: MaybeSend,
    T::Error: Into<tonic::codegen::StdError>,
    T::ResponseBody: http_body::Body<Data = bytes::Bytes> + Send + 'static,
    <T::ResponseBody as http_body::Body>::Error: Into<tonic::codegen::StdError> + Send
{
    fn auth_server(&self) -> &AuthServer<T>;

    async fn handle(&self, effect: RpcOperation) -> RpcOperationOutput {
        match effect {
            RpcOperation::GetSignInUrl(device_info) => {
                let response = self.auth_server().request_signin_url(device_info).await;
                match response {
                    Ok(url) => RpcOperationOutput::SignInUrl(url),
                    Err(e) => RpcOperationOutput::NetworkError(e.into())
                }
            }
            RpcOperation::GetMe() => {
                let response = self.auth_server().get_me().await;
                match response {
                    Ok(user) => RpcOperationOutput::GetMe(user),
                    Err(e) => RpcOperationOutput::NetworkError(e.into())
                }
            }
        }
    }
}
