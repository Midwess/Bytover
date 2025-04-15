use chrono::Utc;
use futures_util::StreamExt;

use crate::app::modules::nearby::NearbyEvent;
use crate::app::modules::transfer::TransferEvent;
use crate::app::operations::device::DeviceOperation;
use crate::app::operations::internet::InternetOperation;
use crate::app::operations::p2p::{P2POperation, P2POperationOutput};
use crate::app::operations::{CoreOperation, CoreOperationOutput};
use crate::app::transfer::target::TransferTarget;
use crate::app::{AppCommandContext, AppEvent};
use crate::entities::peer::Peer;
use crate::entities::user::User;

pub struct NearbyService {}

impl NearbyService {
    pub async fn start_service(&'static self, user: Option<User>, ctx: AppCommandContext) {
        let device = DeviceOperation::get_device_info().into_future(ctx.clone()).await;
        let Ok(current_ip) = InternetOperation::get_current_ip_address().into_future(ctx.clone()).await else {
            log::error!(target: "nearby", "Failed to get current ip address, skip starting nearby service");
            return;
        };

        ctx.request_from_shell(CoreOperation::Notified(AppEvent::Nearby(NearbyEvent::OnIpAddressUpdated(
            current_ip.clone()
        ))))
        .await;

        log::info!(target: "nearby", "Current ip = {current_ip}");
        let ip_parts: String = current_ip
            .split('.')
            .map(|part| part.parse::<i64>().unwrap_or(0).to_string())
            .fold(String::new(), |acc, part| format!("{}{}", acc, part));

        let current_mics = Utc::now().timestamp_micros();
        let peer_id = format!("{}{}", current_mics, ip_parts);

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
                avatar_url: "https://cdn.devlog.studio/public/animal_avatars/Cat.jpg".to_string(),
                email: None,
                device
            }
        };

        let start_p2p_server_request = CoreOperation::P2P(P2POperation::StartNearbyServer(peer));
        let mut start_p2p_server_stream = ctx.stream_from_shell(start_p2p_server_request);

        while let Some(output) = start_p2p_server_stream.next().await {
            match output {
                CoreOperationOutput::P2P(P2POperationOutput::PeerConnected(peer)) => {
                    log::info!(target: "nearby", "New peer connected: {}", peer.id);

                    ctx.send_event(AppEvent::Nearby(NearbyEvent::UpdateNearbyPeers {
                        new: vec![peer.clone()],
                        removed: vec![]
                    }));

                    ctx.send_event(AppEvent::Transfer(TransferEvent::UpdateTransferTargets {
                        new: vec![TransferTarget::Nearby(peer.clone())],
                        removed: vec![]
                    }));

                    ctx.spawn(|it| async move {
                        self.handle_peer_connection(peer, it).await;
                    });

                    ctx.request_from_shell(CoreOperation::Render).await;
                }
                CoreOperationOutput::DeviceError(error) => {
                    log::error!(target: "nearby", "Device error: {:?}", error);
                    ctx.send_event(AppEvent::Nearby(NearbyEvent::ClearNearbyPeers));
                    ctx.request_from_shell(CoreOperation::Render).await;
                    break;
                }
                CoreOperationOutput::P2P(P2POperationOutput::NearbyServerStopped) => {
                    log::info!(target: "nearby", "Nearby server stopped");
                    ctx.send_event(AppEvent::Nearby(NearbyEvent::ClearNearbyPeers));
                    ctx.request_from_shell(CoreOperation::Render).await;
                    break;
                }
                _ => {
                    panic!("Unexpected output from nearby server, output: {:?}", output);
                }
            }
        }
    }

    pub async fn handle_peer_connection(&'static self, peer: Peer, ctx: AppCommandContext) {
        let ns = format!("peer-id+{}", peer.id);
        log::info!(target: ns.as_str(), "Handle peer connection: {}", peer.id);
        let request = CoreOperation::P2P(P2POperation::PeerEvents(peer.id.clone()));
        let mut stream = ctx.stream_from_shell(request);

        while let Some(output) = stream.next().await {
            match output {
                CoreOperationOutput::P2P(P2POperationOutput::PeerDisconnected()) => {
                    break;
                }
                CoreOperationOutput::P2P(P2POperationOutput::ReceivedSessionRequest {
                    request_id,
                    remote_session
                }) => {
                    log::info!(target: ns.as_str(), "Received session request from peer: {}", peer.id);
                    let request = AppEvent::Transfer(TransferEvent::TransferRequest {
                        request_id,
                        remote_session,
                        peer: peer.clone()
                    });
                    ctx.notify_shell(CoreOperation::Notified(request));
                }
                CoreOperationOutput::ConnectionError(error) => {
                    log::error!(target: ns.as_str(), "Connection error: {:?}", error);
                    break;
                }
                CoreOperationOutput::DeviceError(error) => {
                    log::error!(target: ns.as_str(), "Device error: {:?}", error);
                    break;
                }
                CoreOperationOutput::Void => {
                    continue;
                }
                _ => {
                    panic!("Unexpected output from nearby server, output: {:?}", output);
                }
            }
        }

        log::info!(target: ns.as_str(), "Peer disconnected: {}", peer.id);

        ctx.send_event(AppEvent::Nearby(NearbyEvent::UpdateNearbyPeers {
            new: vec![],
            removed: vec![peer.clone()]
        }));

        ctx.send_event(AppEvent::Transfer(TransferEvent::UpdateTransferTargets {
            new: vec![],
            removed: vec![TransferTarget::Nearby(peer.clone())]
        }));

        ctx.request_from_shell(CoreOperation::Render).await;
    }
}
