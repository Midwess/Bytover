use shared::app::operations::internet::InternetOperation;
use shared::app::operations::{CoreOperation, CoreOperationOutput};
use shared::shell::api::network::InternetConnection;
use shared::shell::api::CoreRequest;
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
    pub async fn handle(
        &self,
        request: CoreRequest,
        effect: CoreOperation,
    ) -> CoreOperationOutput {
        match effect {
            CoreOperation::Rpc(rpc_effect) => {
                let response = self.rpc.handle(rpc_effect).await;
                CoreOperationOutput::Rpc(response)
            }
            CoreOperation::Persistent(database) => self.persistent.handle(database).await.into(),
            CoreOperation::Transfer(transfer) => self.transfer.handle(request, transfer).await.into(),
            CoreOperation::Internet(internet) => match internet {
                InternetOperation::Locate(geolocation) => match self.internet_connection.locate(geolocation).await {
                    Ok(net) => CoreOperationOutput::FindingScopes(net.finding_scopes()),
                    Err(error) => CoreOperationOutput::Error(error)
                }
            },
            CoreOperation::P2P(p2p) => self.p2p.handle(request, p2p).await.into(),
            CoreOperation::Delay(duration) => {
                sleep(duration).await;
                CoreOperationOutput::None
            }
            _ => panic!("Native executor doesn't support this effect {effect:?}")
        }
    }
}
