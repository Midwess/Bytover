use crate::app::operations::rpc::{RpcOperation, RpcOperationOutput};
use crate::app::operations::CoreOperationOutput;
use crate::errors::CoreError;
use crate::protocol::rpc::app_server::AppServer;
use crate::protocol::rpc::cloud_server::CloudServer;
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
    <T::ResponseBody as http_body::Body>::Error: Into<tonic::codegen::StdError> + Send,
{
    fn app_server(&self) -> &AppServer<T>;
    fn cloud_server(&self) -> &CloudServer<T>;

    async fn handle(&self, effect: RpcOperation) -> Result<CoreOperationOutput, CoreError> {
        match effect {
            RpcOperation::GetAuthenticateUrl(device_info) => {
                let response = self.app_server().authenticate(device_info).await?;
                Ok(response.into())
            }
            RpcOperation::GetMe() => {
                let (user, device_unique_key) = self.app_server().get_me().await?;
                Ok(RpcOperationOutput::GetMe { user, device_unique_key }.into())
            }
            RpcOperation::GetUserById(user_id) => {
                let response = self.app_server().get_user_by_id(user_id).await?;
                Ok(RpcOperationOutput::GetUserById(response).into())
            }
            RpcOperation::Feedback { email, message } => {
                self.app_server().feedback(email, message).await?;
                Ok(CoreOperationOutput::None)
            }
            RpcOperation::CreateP2PSession {
                alias,
                signalling_key,
                signalling_route,
            } => {
                let p2p_session = self.app_server().create_device_session(alias, signalling_key, signalling_route).await?;
                Ok(CoreOperationOutput::P2PSession(p2p_session))
            }
            RpcOperation::GetDeviceAliases => {
                let aliases = self.app_server().get_device_aliases().await?;
                Ok(CoreOperationOutput::DeviceAliases(aliases))
            }
            RpcOperation::GenPeer { device } => {
                let peer = self.app_server().gen_peer(device).await?;
                Ok(CoreOperationOutput::Rpc(RpcOperationOutput::GenPeer(peer)))
            }
            RpcOperation::GetCapabilities => {
                let caps = self.cloud_server().get_capabilities().await?;
                Ok(CoreOperationOutput::Rpc(RpcOperationOutput::GetCapabilities(caps)))
            }
            RpcOperation::SubmitStoreKitTransaction {
                transaction_id,
                product_id,
            } => {
                let outcome = self
                    .cloud_server()
                    .submit_storekit_transaction(transaction_id, product_id)
                    .await?;
                Ok(CoreOperationOutput::Rpc(RpcOperationOutput::SubmitStoreKitTransaction(outcome)))
            }
        }
    }
}
