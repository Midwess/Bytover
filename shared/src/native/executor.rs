use crate::{app::{operations::{rpc::{RpcOperation, RpcOperationOutput}, CoreOperation, CoreOperationOutput}, AppEvent}, di_container::DiContainer, process_event};

// Handle the effect comming from the platform
// This is the placed where we can put Rust logic to share accross platform
pub struct NativeExecutor {}

impl NativeExecutor {
    pub async fn handle(&self, effect: CoreOperation) -> CoreOperationOutput {
        match effect {
            CoreOperation::Rpc(RpcOperation::GetSignInUrl(device_info)) => {
                let di_container = DiContainer::get_instance();
                let response = di_container.get_authentication_server().request_signin_url(device_info).await.unwrap();
                CoreOperationOutput::Rpc(RpcOperationOutput::SignInUrl(response))
            },
            CoreOperation::Void => {
                log::info!(target: "Tiendang-debug", "Handling void event");
                process_event(&crate::serialize(&AppEvent::Void));
                CoreOperationOutput::Void
            }
            _ => panic!("Native executor doesn't support this effect {:?}", effect)
        }
    }
}
