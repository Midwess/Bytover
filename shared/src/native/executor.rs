use crate::app::operations::{CoreOperation, CoreOperationOutput};
use crate::app::AppEvent;
use crate::process_event;

use super::database::NativeDatabase;
use super::rpc::NativeRpc;
use super::local_storage::NativeLocalStorage;
// Handle the effect comming from the platform
// This is the placed where we can put Rust logic to share accross platform
pub struct NativeExecutor {
    pub rpc: NativeRpc,
    pub database: NativeDatabase,
    pub local_storage: NativeLocalStorage
}

impl NativeExecutor {
    pub async fn handle(&self, effect: CoreOperation) -> CoreOperationOutput {
        match effect {
            CoreOperation::Rpc(rpc_effect) => {
                let response = self.rpc.handle(rpc_effect).await;
                CoreOperationOutput::Rpc(response)
            }
            CoreOperation::Void => {
                process_event(&crate::serialize(&AppEvent::Void));
                CoreOperationOutput::Void
            }
            CoreOperation::Database(database) => {
                let response = self.database.handle(database).await;
                CoreOperationOutput::Database(response)
            }
            CoreOperation::LocalStorage(local_storage) => {
                let response = self.local_storage.handle(local_storage).await;
                CoreOperationOutput::LocalStorage(response)
            }
            _ => panic!("Native executor doesn't support this effect {:?}", effect)
        }
    }
}
