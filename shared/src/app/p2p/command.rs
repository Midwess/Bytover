use std::time::Duration;

use crate::app::core::command::AppCommand;
use crate::app::core::extensions::CoreCommandContextUtils;
use crate::app::operations::device::DeviceOperation;
use crate::app::operations::dialog::DialogOperation;
use crate::app::operations::p2p::{P2POperation, P2POperationOutput};
use crate::app::operations::rpc::RpcOperation;
use crate::app::operations::CoreOperationOutput;
use crate::app::p2p::module::P2PEvent;
use crate::app::transfer::module::TransferEvent;
use crate::entities::peer::Peer;
use crate::errors::CoreError;
use crate::CoreOperation;
use futures_util::StreamExt;

impl AppCommand {
    pub async fn restart_nearby(&self) -> Result<(), CoreError> {
        self.run(P2POperation::stop()).await?;
        self.notify_event(P2PEvent::Launch);
        Ok(())
    }

    pub async fn gen_peer(&self, user: Option<crate::entities::user::User>, device: crate::entities::device::DeviceInfo) -> Peer {
        let peer_id = Peer::compute_id(&device.unique_id, user.as_ref().map(|u| u.id).unwrap_or(0));
        match user {
            Some(user) => Peer {
                id: peer_id,
                name: Some(user.name),
                avatar_url: user.avatar,
                email: Some(user.email),
                device,
            },
            None => Peer {
                id: peer_id,
                name: Some(device.name.clone()),
                avatar_url: self.run(RpcOperation::random_avatar()).await.unwrap_or_default(),
                email: None,
                device,
            }
        }
    }

    pub async fn start_nearby_server(&self) -> Result<(), CoreError> {
        let user = RpcOperation::get_me().into_future(self.ctx()).await?;

        let is_already_running = self.run(P2POperation::is_running()).await;
        if is_already_running.unwrap_or(false) {
            log::info!("Nearby server is already running");
            return Ok(());
        }

        let Some(device) = self.run(DeviceOperation::get_device_info()).await else {
            self.run(DialogOperation::toast("Device not found".to_string())).await;
            return Ok(());
        };

        let peer = self.gen_peer(Some(user), device).await;
        self.update_model(P2PEvent::UpdateMe { new_peer: peer.clone() });
        log::info!(target: "nearby", "Starting nearby server with peer {peer:?}");

        let start_p2p_server_request = P2POperation::StartNearbyServer(peer);
        let mut start_p2p_server_stream = self.stream_from_shell(start_p2p_server_request.into());

        let _ = self.run(P2POperation::stop()).await;
        while let Some(output) = start_p2p_server_stream.next().await {
            match output {
                CoreOperationOutput::Error(error) => {
                    log::error!("Nearby server has been stopped: {error:?}, will restart in 3s...");
                    let _ = self.run(P2POperation::stop()).await;
                    self.request_from_shell(CoreOperation::Delay(Duration::from_secs(3))).await;
                    self.notify_event(P2PEvent::Launch);
                    break;
                }
                CoreOperationOutput::P2P(P2POperationOutput::AlreadyRunning) => {
                    log::info!(target: "nearby", "Nearby server already running");
                    break;
                }
                CoreOperationOutput::P2P(P2POperationOutput::NearbyServerStopped) => {
                    log::info!(target: "nearby", "Nearby server stopped");
                    break;
                }
                CoreOperationOutput::None => {}
                _ => {
                    panic!("Unexpected output from nearby server, output: {output:?}");
                }
            }
        };
        Ok(())
    }

}
