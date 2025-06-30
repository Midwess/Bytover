use std::sync::OnceLock;
use std::time::Duration;

use futures_util::StreamExt;
use ulid::Ulid;

use crate::app::core_utils::CoreCommandContextUtils;
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
    pub fn instance() -> &'static NearbyService {
        static INSTANCE: OnceLock<NearbyService> = OnceLock::new();
        INSTANCE.get_or_init(|| NearbyService {})
    }

    pub async fn start_service(&'static self, user: Option<User>, ctx: AppCommandContext) {
        let device = DeviceOperation::get_device_info().into_future(ctx.clone()).await;

        let peer_id = Ulid::new().0.to_string();

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

        ctx.notify_event(AppEvent::Nearby(NearbyEvent::UpdateMe { new: peer.clone() }));

        let start_p2p_server_request = CoreOperation::P2P(P2POperation::StartNearbyServer(peer));
        let mut start_p2p_server_stream = ctx.stream_from_shell(start_p2p_server_request);

        log::info!(target: "nearby", "Starting nearby server");
        while let Some(output) = start_p2p_server_stream.next().await {
            match output {
                CoreOperationOutput::P2P(P2POperationOutput::PeerConnected(peer)) => {
                    log::info!(target: "nearby", "New peer connected: {}", peer.id);

                    ctx.notify_event(AppEvent::Nearby(NearbyEvent::UpdateNearbyPeers {
                        new: vec![peer.clone()],
                        removed: vec![]
                    }));

                    ctx.notify_event(AppEvent::Transfer(TransferEvent::UpdateTransferTargets {
                        new: vec![TransferTarget::Nearby(peer.clone())],
                        removed: vec![]
                    }));

                    ctx.spawn(|it| async move {
                        self.handle_peer_connection(peer, it).await;
                    });
                }
                CoreOperationOutput::DeviceError(error) => {
                    log::error!(target: "nearby", "Device error: {error:?}");
                    ctx.notify_event(AppEvent::Nearby(NearbyEvent::ClearNearbyPeers));
                    break;
                }
                CoreOperationOutput::P2P(P2POperationOutput::NearbyServerStopped) => {
                    log::info!(target: "nearby", "Nearby server stopped");
                    ctx.notify_event(AppEvent::Nearby(NearbyEvent::ClearNearbyPeers));
                    break;
                }
                _ => {
                    panic!("Unexpected output from nearby server, output: {output:?}");
                }
            }
        }
    }

    pub async fn start_ip_address_monitor(&'static self, ctx: AppCommandContext) {
        loop {
            let ip_address = InternetOperation::get_current_ip_address().into_future(ctx.clone()).await;
            if let Ok(ip_address) = ip_address {
                ctx.notify_event(AppEvent::Nearby(NearbyEvent::OnIpAddressUpdated(ip_address)));
            }

            ctx.request_from_shell(CoreOperation::Delay(Duration::from_secs(5))).await;
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
                CoreOperationOutput::P2P(P2POperationOutput::CancelSessionRequest { session_id, .. }) => {
                    log::info!(target: ns.as_str(), "Received cancel session request from peer: {}", peer.id);
                    let request = AppEvent::Transfer(TransferEvent::TransferCanceled { session_id });

                    ctx.notify_shell(CoreOperation::Notified(request));
                }
                CoreOperationOutput::P2P(P2POperationOutput::ReceivedSessionRequest {
                    remote_session
                }) => {
                    log::info!(target: ns.as_str(), "Received session request from peer: {}", peer.id);
                    let request = AppEvent::Transfer(TransferEvent::TransferRequest {
                        remote_session,
                        peer: peer.clone()
                    });
                    ctx.notify_shell(CoreOperation::Notified(request));
                }
                CoreOperationOutput::ConnectionError(error) => {
                    log::error!(target: ns.as_str(), "Connection error: {error:?}");
                    break;
                }
                CoreOperationOutput::DeviceError(error) => {
                    log::error!(target: ns.as_str(), "Device error: {error:?}");
                    break;
                }
                CoreOperationOutput::P2P(P2POperationOutput::ThumbnailFullfillment {
                    session_id,
                    resource_id,
                    path
                }) => {
                    log::info!(target: ns.as_str(), "Received thumbnail fullfillment from peer: {}", peer.id);
                    let request = AppEvent::Transfer(TransferEvent::SessionResourceThumbnailFullfillment {
                        session_id,
                        resource_id,
                        path: path.clone()
                    });
                    ctx.notify_shell(CoreOperation::Notified(request));
                }
                CoreOperationOutput::Void => {
                    continue;
                }
                _ => {
                    panic!("Unexpected output from nearby server, output: {output:?}");
                }
            }
        }

        log::info!(target: ns.as_str(), "Peer disconnected: {}", peer.id);

        ctx.notify_event(AppEvent::Nearby(NearbyEvent::UpdateNearbyPeers {
            new: vec![],
            removed: vec![peer.clone()]
        }));

        ctx.notify_event(AppEvent::Transfer(TransferEvent::UpdateTransferTargets {
            new: vec![],
            removed: vec![TransferTarget::Nearby(peer.clone())]
        }));
    }
}
