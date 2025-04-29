use std::sync::Arc;
use std::time::Duration;

use tokio::time::sleep;

use crate::app::operations::internet::{InternetOperation, InternetOperationOutput};
use crate::app::operations::{CoreOperation, CoreOperationOutput};
use crate::app::AppEvent;
use crate::network::module::InternetConnection;
use crate::{process_event, serialize, ShellRuntime};

use super::database::NativeDatabase;
use super::local_storage::NativeLocalStorage;
use super::message_to_shell::MessageToShell;
use super::p2p::P2PNativeExecutor;
use super::rpc::NativeRpc;
use super::transfer::TransferNative;
// Handle the effect comming from the platform
// This is the placed where we can put Rust logic to share accross platform
pub struct NativeExecutor {
    pub rpc: NativeRpc,
    pub database: NativeDatabase,
    pub local_storage: NativeLocalStorage,
    pub transfer: TransferNative,
    pub p2p: P2PNativeExecutor
}

impl NativeExecutor {
    pub async fn handle(&self, request_id: u32, effect: CoreOperation, shell_runtime: Arc<dyn ShellRuntime>) -> CoreOperationOutput {
        self.transfer.update_shell_runtime(&shell_runtime);
        self.p2p.update_shell_runtime(&shell_runtime);

        match effect {
            CoreOperation::Rpc(rpc_effect) => {
                let response = self.rpc.handle(rpc_effect).await;
                CoreOperationOutput::Rpc(response)
            }
            CoreOperation::Void => {
                process_event(crate::serialize(&AppEvent::Void));
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
            CoreOperation::Transfer(transfer) => self.transfer.handle(request_id, transfer).await,
            CoreOperation::Internet(internet) => match internet {
                InternetOperation::GetCurrentIpAddress => {
                    let internet_connection = InternetConnection::new();
                    match internet_connection.ip_address().await {
                        Ok(ip_address) => CoreOperationOutput::Internet(InternetOperationOutput::GetCurrentIpAddress(ip_address)),
                        Err(error) => CoreOperationOutput::Internet(InternetOperationOutput::NetworkError(error))
                    }
                }
            },
            CoreOperation::P2P(p2p) => self.p2p.handle(request_id, p2p).await,
            CoreOperation::Delay(duration) => {
                sleep(duration).await;
                CoreOperationOutput::Delay()
            }
            _ => panic!("Native executor doesn't support this effect {:?}", effect)
        }
    }
}
