use std::time::Duration;

use crate::app::core::command::AppCommand;
use crate::app::core::extensions::CoreCommandContextUtils;
use crate::app::transfer::module::TransferEvent;
use crate::app::nearby::module::NearbyEvent;
use crate::app::operations::device::DeviceOperation;
use crate::app::operations::internet::InternetOperation;
use crate::app::operations::p2p::{P2POperation, P2POperationOutput};
use crate::app::operations::{CoreOperation, CoreOperationOutput};
use crate::app::AppEvent;
use crate::entities::peer::Peer;
use crate::entities::target::TransferTarget;
use crate::entities::user::User;
use futures_util::StreamExt;
use uuid::Uuid;

impl AppCommand {
    pub async fn receive_nearby_events(&self, user: Option<User>) {
        let device = DeviceOperation::get_device_info().into_future(self.ctx()).await;

        let peer_id = Uuid::now_v7().to_string();

        let peer = match user {
            Some(user) => Peer {
                id: peer_id.clone(),
                name: Some(user.name),
                avatar_url: user.avatar,
                email: Some(user.email),
                device
            },
            None => Peer {
                id: peer_id.clone(),
                name: None,
                avatar_url: Peer::random_avatar(),
                email: None,
                device
            }
        };

        self.notify_event(AppEvent::Nearby(NearbyEvent::UpdateMe { new_peer: peer.clone() }));

        let start_p2p_server_request = P2POperation::StartNearbyServer(peer);
        let mut start_p2p_server_stream = self.stream_from_shell(start_p2p_server_request);

        while let Some(output) = start_p2p_server_stream.next().await {
            match output {
                CoreOperationOutput::P2P(P2POperationOutput::PeerConnected(peer)) => {
                    log::info!(target: "nearby", "New peer connected: {}", peer.id);

                    self.notify_event(AppEvent::Nearby(NearbyEvent::UpdateNearbyPeers {
                        new_peer: vec![peer.clone()],
                        removed: vec![]
                    }));

                    self.notify_event(AppEvent::Transfer(TransferEvent::UpdateTransferTargets {
                        added: vec![TransferTarget::Nearby(peer.clone())],
                        removed: vec![]
                    }));

                    self.spawn(|it| async move {
                        it.app().handle_peer_connection(peer).await;
                    });
                }
                CoreOperationOutput::DeviceError(error) => {
                    log::error!(target: "nearby", "Device error: {error:?}");
                    self.notify_event(AppEvent::Nearby(NearbyEvent::ClearNearbyPeers));
                    break;
                }
                CoreOperationOutput::P2P(P2POperationOutput::NearbyServerStopped) => {
                    log::info!(target: "nearby", "Nearby server stopped");
                    self.notify_event(AppEvent::Nearby(NearbyEvent::ClearNearbyPeers));
                    break;
                }
                CoreOperationOutput::Void => {}
                CoreOperationOutput::ConnectionError(error) => {
                    log::error!(target: "nearby", "Connection error: {error:?}");
                    self.notify_event(AppEvent::Nearby(NearbyEvent::ClearNearbyPeers));
                    break;
                }
                _ => {
                    panic!("Unexpected output from nearby server, output: {output:?}");
                }
            }
        }
    }

    pub async fn start_locator_monitor(&self) {
        loop {
            let geo_location = self.run(DeviceOperation::get_geo_location()).await;
            let scopes = self.run(InternetOperation::locate(geo_location)).await;
            if let Ok(scopes) = scopes {
                self.request(P2POperation::UpdateFindingScopes(scopes)).await;
                log::info!(target: "nearby", "Updated scope");
            }

            self.request(Duration::from_secs(5)).await;
        }
    }

    pub async fn handle_peer_connection(&self, peer: Peer) {
        let request = P2POperation::PeerEvents(peer.id.clone());
        let mut stream = self.stream_from_shell(request);

        while let Some(output) = stream.next().await {
            match output {
                CoreOperationOutput::P2P(P2POperationOutput::PeerDisconnected()) => {
                    break;
                }
                CoreOperationOutput::P2P(P2POperationOutput::CancelSessionRequest { session_id, .. }) => {
                    let request = AppEvent::Transfer(TransferEvent::TransferCanceled { session_id });

                    self.notify_shell(CoreOperation::Notified(request));
                }
                CoreOperationOutput::P2P(P2POperationOutput::ReceivedSessionRequest { remote_session }) => {
                    let request = AppEvent::Transfer(TransferEvent::TransferRequest {
                        remote_session,
                        peer: peer.clone()
                    });
                    self.notify_shell(CoreOperation::Notified(request));
                }
                CoreOperationOutput::ConnectionError(error) => {
                    log::error!("Connection error: {error:?}");
                    break;
                }
                CoreOperationOutput::DeviceError(error) => {
                    log::error!("Device error: {error:?}");
                    break;
                }
                CoreOperationOutput::Void => {
                    continue;
                }
                _ => {
                    panic!("Unexpected output from nearby server, output: {output:?}");
                }
            }
        }

        self.notify_event(AppEvent::Nearby(NearbyEvent::UpdateNearbyPeers {
            new_peer: vec![],
            removed: vec![peer.clone()]
        }));

        self.notify_event(AppEvent::Transfer(TransferEvent::UpdateTransferTargets {
            added: vec![],
            removed: vec![TransferTarget::Nearby(peer.clone())]
        }));
    }
}
