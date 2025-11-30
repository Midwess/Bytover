use crate::app::operations::rpc::{RpcOperation, RpcOperationOutput};
use crate::app::operations::CoreOperationOutput;
use crate::errors::CoreError;
use crate::protocol::rpc::auth_server::AppServer;
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
    fn app_server(&self) -> &AppServer<T>;

    async fn handle(&self, effect: RpcOperation) -> Result<CoreOperationOutput, CoreError> {
        match effect {
            RpcOperation::GetAuthenticateUrl(device_info) => {
                let response = self.app_server().authenticate(device_info).await?;
                Ok(response.into())
            }
            RpcOperation::GetMe() => {
                let response = self.app_server().get_me().await?;
                Ok(RpcOperationOutput::GetMe(response).into())
            }
            RpcOperation::Feedback { email, message } => {
                self.app_server().feedback(email, message).await?;
                Ok(CoreOperationOutput::None)
            }
            RpcOperation::RandomAvatar => {
                let avatar = self.app_server().random_avatar().await?;
                Ok(CoreOperationOutput::String(avatar))
            }
        }
    }
}
