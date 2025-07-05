use std::sync::Arc;

use tokio::time::sleep;

use super::p2p::P2PNativeExecutor;
use super::persistent::NativePersistent;
use super::rpc::NativeRpc;
use super::transfer::TransferNative;
use crate::{process_event, ShellRuntime};
use shared::app::operations::internet::{InternetOperation, InternetOperationOutput};
use shared::app::operations::{CoreOperation, CoreOperationOutput};
use shared::app::AppEvent;
use shared::core_api::network::InternetConnection;
// Handle the effect comming from the platform
// This is the placed where we can put Rust logic to share accross platform
pub struct NativeExecutor {
    pub rpc: NativeRpc,
    pub persistent: NativePersistent,
    pub transfer: TransferNative,
    pub p2p: P2PNativeExecutor
}

impl NativeExecutor {
    pub async fn handle(&self, request_id: u32, effect: CoreOperation, shell_runtime: Arc<dyn ShellRuntime>) -> CoreOperationOutput {
        match effect {
            CoreOperation::Rpc(rpc_effect) => {
                let response = self.rpc.handle(rpc_effect).await;
                CoreOperationOutput::Rpc(response)
            }
            CoreOperation::Void => {
                process_event(crate::serialize(&AppEvent::Void));
                CoreOperationOutput::Void
            }
            CoreOperation::Persistent(database) => {
                let response = self.persistent.handle(database).await;
                CoreOperationOutput::Database(response)
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
            _ => panic!("Native executor doesn't support this effect {effect:?}")
        }
    }
}
