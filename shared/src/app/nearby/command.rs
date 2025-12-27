use std::time::Duration;

use crate::app::core::command::AppCommand;
use crate::app::core::extensions::CoreCommandContextUtils;
use crate::app::nearby::module::NearbyEvent;
use crate::app::operations::device::DeviceOperation;
use crate::app::operations::dialog::DialogOperation;
use crate::app::operations::internet::InternetOperation;
use crate::app::operations::p2p::{P2POperation, P2POperationOutput};
use crate::app::operations::CoreOperationOutput;
use crate::app::transfer::module::TransferEvent;
use crate::entities::peer::Peer;
use crate::entities::user::User;
use futures_util::StreamExt;
use uuid::Uuid;
use crate::app::operations::rpc::RpcOperation;
use crate::CoreOperation;
use crate::entities::device::DeviceInfo;
use crate::errors::CoreError;

impl AppCommand {
    pub async fn restart_nearby(&self, auto_launch: bool) -> Result<(), CoreError> {
        self.run(P2POperation::stop()).await?;
        self.notify_event(NearbyEvent::Launch {auto_launch});

        Ok(())
    }

    pub async fn gen_peer(&self, user: Option<User>, device: DeviceInfo) -> Peer {
        let peer_id = Uuid::now_v7().to_string();

        match user {
            Some(user) => Peer {
                id: peer_id.clone(),
                name: Some(user.name),
                avatar_url: user.avatar,
                email: Some(user.email),
                device
            },
            None => Peer {
                id: peer_id.clone(),
                name: Some(device.name.clone()),
                avatar_url: self.run(RpcOperation::random_avatar()).await.unwrap_or_default(),
                email: None,
                device
            }
        }
    }

    pub async fn start_nearby_server(&self, auto_launch: bool) {
        let user = RpcOperation::get_me().into_future(self.ctx()).await.ok();

        let is_already_running = self.run(P2POperation::is_running()).await;
        if is_already_running.unwrap_or(false) {
            log::info!("Nearby server is already running");
            return;
        }

        let Some(device) = self.run(DeviceOperation::get_device_info()).await else {
            self.run(DialogOperation::toast("Device not found".to_string())).await;
            return;
        };

        let peer = self.gen_peer(user, device).await;

        self.update_model(NearbyEvent::UpdateMe { new_peer: peer.clone() });
        let start_p2p_server_request = P2POperation::StartNearbyServer(peer);
        let mut start_p2p_server_stream = self.stream_from_shell(start_p2p_server_request.into());

        let _ = self.run(P2POperation::stop()).await;
        log::info!(target: "nearby", "Starting nearby server");
        while let Some(output) = start_p2p_server_stream.next().await {
            match output {
                CoreOperationOutput::P2P(P2POperationOutput::PeerConnected(peer)) => {
                    log::info!(target: "nearby", "New peer connected: {}", peer.id);

                    self.notify_event(NearbyEvent::UpdateNearbyPeers {
                        new_peer: vec![peer.clone()],
                        removed: vec![]
                    });

                    self.spawn(|it| async move {
                        it.app().handle_peer_connection(peer).await;
                    });
                }
                CoreOperationOutput::Error(error) => {
                    log::error!("Nearby server has been stopped: {error:?}, will restart in 3s...");
                    self.notify_event(NearbyEvent::ClearNearbyPeers);
                    let _ = self.run(P2POperation::stop()).await;

                    self.request_from_shell(CoreOperation::Delay(Duration::from_secs(3))).await;
                    self.notify_event(NearbyEvent::Launch {auto_launch});
                    break;
                }
                CoreOperationOutput::P2P(P2POperationOutput::AlreadyRunning) => {
                    log::info!(target: "nearby", "Nearby server already running");
                    break;
                }
                CoreOperationOutput::P2P(P2POperationOutput::NearbyServerStopped) => {
                    log::info!(target: "nearby", "Nearby server stopped, stop server");
                    self.notify_event(NearbyEvent::ClearNearbyPeers);
                    break;
                }
                CoreOperationOutput::None => {}
                _ => {
                    panic!("Unexpected output from nearby server, output: {output:?}");
                }
            }
        }
    }

    pub async fn start_locator_monitor(&self) {
        loop {
            let geo_location = self.run(DeviceOperation::get_geo_location()).await;
            let delay = match geo_location {
                Some(_) => Duration::from_secs(15),
                None => Duration::from_secs(5)
            };
            let scopes = self.run(InternetOperation::locate(geo_location)).await;
            if let Ok(scopes) = scopes {
                let _ = self.run(P2POperation::update_finding_scopes(scopes)).await;
            }

            self.request(delay).await;
        }
    }

    pub async fn handle_peer_connection(&self, peer: Peer) {
        let request = P2POperation::PeerEvents(peer.id.clone());
        let mut stream = self.stream_from_shell(request.into());

        while let Some(output) = stream.next().await {
            match output {
                CoreOperationOutput::P2P(P2POperationOutput::PeerDisconnected()) => {
                    log::info!("Peer disconnected: {}", peer.id);
                    break;
                }
                CoreOperationOutput::P2P(P2POperationOutput::CancelSessionRequest { session_id, .. }) => {
                    self.notify_event(TransferEvent::TransferCanceled { session_id });
                }
                CoreOperationOutput::P2P(P2POperationOutput::ReceivedSessionsOverview { peer_id, sessions }) => {
                    self.notify_event(TransferEvent::ReceivedSessionsOverview { peer_id, sessions });
                }
                CoreOperationOutput::P2P(P2POperationOutput::ReceivedViewSessionRequest { peer_id, request_id, order_id, password }) => {
                    self.notify_event(TransferEvent::ReceivedViewSessionRequest { peer_id, request_id, order_id, password });
                }
                CoreOperationOutput::P2P(P2POperationOutput::ReceivedDownloadRequest { peer_id, session_order_id, resource_order_id, transfer_id }) => {
                    self.notify_event(TransferEvent::ReceivedDownloadRequest { peer_id, session_order_id, resource_order_id, transfer_id });
                }
                CoreOperationOutput::P2P(P2POperationOutput::NearbyServerStopped) => {
                    log::info!("Nearby server stopped, stop peer connection");
                    break;
                }
                CoreOperationOutput::Error(error) => {
                    log::error!("Connection error: {error:?}");
                    break;
                }
                CoreOperationOutput::None => {
                    continue;
                }
                _ => {
                    log::warn!("Unexpected output from nearby server, output: {output:?}");
                }
            }
        }

        self.notify_event(NearbyEvent::UpdateNearbyPeers {
            new_peer: vec![],
            removed: vec![peer.clone()]
        });
    }
}
