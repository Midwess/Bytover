use shared::shell::executor::p2p::P2PNativeExecutor;

pub struct P2PNativeExecutorImpl;

#[async_trait::async_trait(?Send)]
impl P2PNativeExecutor for P2PNativeExecutorImpl {
    async fn handle(&self, _request: shared::shell::api::CoreRequest, effect: shared::app::operations::p2p::P2POperation) -> Result<shared::app::operations::CoreOperationOutput, shared::errors::CoreError> {
        match effect {
            shared::app::operations::p2p::P2POperation::ViewSessionDetail { .. } => {
                todo!("ViewSessionDetail is not yet implemented on WASM")
            }
            shared::app::operations::p2p::P2POperation::DownloadResource { .. } => {
                todo!("DownloadResource is not yet implemented on WASM")
            }
            shared::app::operations::p2p::P2POperation::DownloadAllResources { .. } => {
                todo!("DownloadAllResources is not yet implemented on WASM")
            }
            shared::app::operations::p2p::P2POperation::StopNearbyServer => {
                todo!("StopNearbyServer is not yet implemented on WASM")
            }
            shared::app::operations::p2p::P2POperation::IsRunning => {
                todo!("IsRunning is not yet implemented on WASM")
            }
            shared::app::operations::p2p::P2POperation::CancelResource { .. } => {
                todo!("CancelResource is not yet implemented on WASM")
            }
            shared::app::operations::p2p::P2POperation::BroadcastCancelSession { .. } => {
                todo!("BroadcastCancelSession is not yet implemented on WASM")
            }
            shared::app::operations::p2p::P2POperation::PeerEvents(_) => {
                todo!("PeerEvents is not yet implemented on WASM")
            }
            shared::app::operations::p2p::P2POperation::StartNearbyServer(_) => {
                todo!("StartNearbyServer is not yet implemented on WASM")
            }
            shared::app::operations::p2p::P2POperation::SendSessionDetail { .. } => {
                todo!("SendSessionDetail is not yet implemented on WASM")
            }
            shared::app::operations::p2p::P2POperation::StreamResourceToPeer { .. } => {
                todo!("StreamResourceToPeer is not yet implemented on WASM")
            }
            shared::app::operations::p2p::P2POperation::SendResourceNotification { .. } => {
                todo!("SendResourceNotification is not yet implemented on WASM")
            }
        }
    }
}
