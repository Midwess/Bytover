use crate::app::core::extensions::CoreCommandContextUtils;
use crate::app::modules::AppModule;
use crate::app::operations::CoreOperation;
use crate::app::view_models::peer::PeerViewModel;
use crate::app::{AppModel, BitBridge};
use crate::entities::device::DeviceInfo;
use crate::entities::finding_scope::FindingScope;
use crate::entities::peer::Peer;
use crux_core::{App, Command};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct NearbyModel {
    pub device: Option<DeviceInfo>,
    pub finding_scopes: Vec<FindingScope>,
    pub me: Option<Peer>,
    pub peers: Vec<Peer>
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct NearbyViewModel {
    pub me: Option<PeerViewModel>,
    pub peers: Vec<PeerViewModel>
}

pub struct NearbyModule;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum NearbyEvent {
    Launch,
    UpdateMe { new_peer: Peer },
    UpdateNearbyPeers { new_peer: Vec<Peer>, removed: Vec<Peer> },
    ClearNearbyPeers
}

impl AppModule<BitBridge> for NearbyModule {
    type Event = NearbyEvent;
    type ViewModel = NearbyViewModel;

    fn update(
        &self,
        event: Self::Event,
        model: &mut AppModel,
        _caps: &<BitBridge as App>::Capabilities
    ) -> Command<<BitBridge as App>::Effect, <BitBridge as App>::Event> {
        match event {
            NearbyEvent::Launch => {
                let user = model.authentication.user.clone();

                Command::all(vec![
                    Command::new(|it| async move {
                        it.app().receive_nearby_events(user).await;
                    }),
                    Command::new(|it| async move {
                        log::info!(target: "nearby", "Starting locator monitor");
                        it.app().start_locator_monitor().await;
                    }),
                ])
            }
            NearbyEvent::UpdateNearbyPeers { new_peer, removed } => {
                model.nearby.peers.retain(|it| !removed.contains(it));
                model.nearby.peers.extend(new_peer);
                Command::new(|it| async move {
                    it.notify_shell(CoreOperation::Render);
                })
            }
            NearbyEvent::ClearNearbyPeers => {
                model.nearby.peers.clear();
                Command::new(|it| async move {
                    it.notify_shell(CoreOperation::Render);
                })
            }
            NearbyEvent::UpdateMe { new_peer } => {
                model.nearby.me = Some(new_peer);
                Command::new(|it| async move {
                    it.notify_shell(CoreOperation::Render);
                })
            }
        }
    }

    fn view(&self, model: &AppModel) -> Self::ViewModel {
        Self::ViewModel {
            me: model.nearby.me.as_ref().map(PeerViewModel::from),
            peers: model.nearby.peers.iter().map(PeerViewModel::from).collect()
        }
    }
}
