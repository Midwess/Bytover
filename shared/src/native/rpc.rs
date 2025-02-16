use crate::{app::operations::rpc::{RpcOperation, RpcOperationOutput}, di_container::DiContainer};

pub struct NativeRpc {}

impl NativeRpc {
    pub async fn handle(&self, effect: RpcOperation) -> RpcOperationOutput {
        match effect {
            RpcOperation::GetSignInUrl(device_info) => {
                let di_container = DiContainer::get_instance();
                let response = di_container.get_authentication_server().request_signin_url(device_info).await.unwrap();
                RpcOperationOutput::SignInUrl(response)
            },
            _ => panic!("Native rpc doesn't support this effect {:?}", effect)
        }
    }
}