use uuid::Uuid;

use crate::app::modules::transfer::TransferEvent;
use crate::app::operations::device::DeviceOperation;
use crate::app::operations::internet::InternetOperation;
use crate::app::operations::transfer::TransferOperation;
use crate::app::operations::CoreOperation;
use crate::app::{AppCommandContext, AppEvent};
use crate::entities::peer::Peer;
use crate::entities::user::User;

pub struct NearbyService {}

impl NearbyService {
    pub async fn init(&self, user: Option<User>, ctx: AppCommandContext) {
        let device = DeviceOperation::get_device_info().into_future(ctx.clone()).await;
        let peer_id = Uuid::new_v4().as_u128().to_string();
        let peer = match user {
            Some(user) => Peer {
                id: peer_id,
                name: Some(user.name),
                avatar_url: user.avatar,
                email: Some(user.email),
                device
            },
            None => Peer {
                id: peer_id,
                name: None,
                avatar_url: "https://cdn.devlog.studio/public/animal_avatars/Cat.jpg".to_string(),
                email: None,
                device
            }
        };

        TransferOperation::start_nearby_server(peer).into_future(ctx.clone()).await;
        if let Ok(local_ip) = InternetOperation::get_current_ip_address().into_future(ctx.clone()).await {
            ctx.send_event(AppEvent::Transfer(TransferEvent::OnIpAddressUpdated(local_ip)));
        }
    }

    pub async fn on_new_peer_come(&self, peer: Peer, ctx: AppCommandContext) {
        let new_peers = vec![peer];
        let removed_peers = vec![];
        ctx.send_event(AppEvent::Transfer(TransferEvent::UpdatePeers { new: new_peers, removed: removed_peers }));
        ctx.request_from_shell(CoreOperation::Render).await;
    }

    pub async fn on_peer_leaved(&self, peer: Peer, ctx: AppCommandContext) {
        let new_peers = vec![];
        let removed_peers = vec![peer];
        ctx.send_event(AppEvent::Transfer(TransferEvent::UpdatePeers { new: new_peers, removed: removed_peers }));
        ctx.request_from_shell(CoreOperation::Render).await;
    }
}
