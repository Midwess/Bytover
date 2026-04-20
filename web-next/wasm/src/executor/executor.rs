use futures_timer::Delay;
use shared::app::operations::{CoreOperation, CoreOperationOutput};
use shared::shell::api::CoreRequest;
use shared::shell::executor::p2p::P2PNativeExecutor;
use shared::shell::executor::persistent::NativePersistent;
use shared::shell::executor::rpc::NativeRpc;
use shared::shell::executor::transfer::TransferNative;
use tonic_web_wasm_client::Client;

// Handle the effect coming from the platform
// This is the placed where we can put Rust logic to share across a platform
pub struct NativeExecutor {
    pub rpc: Box<dyn NativeRpc<Client>>,
    pub persistent: Box<dyn NativePersistent>,
    pub transfer: Box<dyn TransferNative<Client>>,
    pub p2p: Box<dyn P2PNativeExecutor>,
}

impl NativeExecutor {
    pub async fn handle(&self, request: CoreRequest, effect: CoreOperation) -> CoreOperationOutput {
        match effect {
            CoreOperation::Rpc(rpc_effect) => {
                let response = self.rpc.handle(rpc_effect).await;
                response.into()
            }
            CoreOperation::Persistent(database) => self.persistent.handle(database).await.into(),
            CoreOperation::Transfer(transfer) => self.transfer.handle(request, transfer).await.into(),
            CoreOperation::P2P(p2p) => self.p2p.handle(request, p2p).await.into(),
            CoreOperation::Delay(duration) => {
                Delay::new(duration).await;
                CoreOperationOutput::None
            }
            _ => panic!("Native executor doesn't support this effect {effect:?}"),
        }
    }
}
