use crate::{app::{operations::{rpc::{RpcOperation, RpcOperationOutput}, CoreOperation, CoreOperationOutput}, AppEvent}, di_container::DiContainer, process_event};

use super::rpc::NativeRpc;

// Handle the effect comming from the platform
// This is the placed where we can put Rust logic to share accross platform
pub struct NativeExecutor {
    pub rpc: NativeRpc
}

impl NativeExecutor {
    pub async fn handle(&self, effect: CoreOperation) -> CoreOperationOutput {
        match effect {
            CoreOperation::Rpc(rpc_effect) => {
                let response = self.rpc.handle(rpc_effect).await;
                CoreOperationOutput::Rpc(response)
            },
            CoreOperation::Void => {
                log::info!(target: "Tiendang-debug", "Handling void event");
                process_event(&crate::serialize(&AppEvent::Void));
                CoreOperationOutput::Void
            },
            CoreOperation::Database(database) => {

            },
            _ => panic!("Native executor doesn't support this effect {:?}", effect)
        }
    }
}
