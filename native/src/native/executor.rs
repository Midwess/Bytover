use std::sync::Arc;

use crate::ShellRuntime;
use shared::app::operations::internet::{InternetOperation, InternetOperationOutput};
use shared::app::operations::{CoreOperation, CoreOperationOutput};
use shared::shell::api::network::InternetConnection;
use shared::shell::executor::p2p::P2PNativeExecutor;
use shared::shell::executor::persistent::NativePersistent;
use shared::shell::executor::rpc::NativeRpc;
use shared::shell::executor::transfer::TransferNative;
use tokio::time::sleep;
use tonic::transport::Channel;

// Handle the effect coming from the platform
// This is the placed where we can put Rust logic to share across platform
pub struct NativeExecutor {
    pub rpc: Box<dyn NativeRpc<Channel>>,
    pub persistent: Box<dyn NativePersistent>,
    pub transfer: Box<dyn TransferNative<Channel>>,
    pub p2p: Box<dyn P2PNativeExecutor>,
    pub internet_connection: InternetConnection
}

impl NativeExecutor {
    pub async fn handle(&self, request_id: u32, effect: CoreOperation, _shell_runtime: Arc<dyn ShellRuntime>) -> CoreOperationOutput {
        match effect {
            CoreOperation::Rpc(rpc_effect) => {
                let response = self.rpc.handle(rpc_effect).await;
                CoreOperationOutput::Rpc(response)
            }
            CoreOperation::Persistent(database) => {
                let response = self.persistent.handle(database).await;
                CoreOperationOutput::Database(response)
            }
            CoreOperation::Transfer(transfer) => self.transfer.handle(request_id, transfer).await,
            CoreOperation::Internet(internet) => match internet {
                InternetOperation::Locate(geolocation) => match self.internet_connection.locate(geolocation).await {
                    Ok(net) => CoreOperationOutput::Internet(InternetOperationOutput::Locate(net.finding_scopes())),
                    Err(error) => CoreOperationOutput::Internet(InternetOperationOutput::NetworkError(error))
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
