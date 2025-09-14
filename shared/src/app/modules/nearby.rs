use crate::app::core_utils::CoreCommandContextUtils;
use crate::app::modules::AppModule;
use crate::app::nearby::finding_scope::FindingScope;
use crate::app::nearby::nearby_services::NearbyService;
use crate::app::operations::CoreOperation;
use crate::app::view_models::peer::PeerViewModel;
use crate::app::{AppEvent, AppModel, BitBridge};
use crate::entities::device::DeviceInfo;
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

pub struct NearbyModule {
    pub nearby_service: &'static NearbyService
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum NearbyEvent {
    Launch(),
    StartLocate,
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
        let nearby_service = self.nearby_service;
        match event {
            NearbyEvent::Launch() => {
                let user = model.authentication.user.clone();

                let nearby_command = Command::new(|it| async move {
                    nearby_service.start_service(user, it.clone()).await;
                });

                Command::all(vec![
                    Command::new(|it| async move {
                        it.notify_event(AppEvent::Nearby(NearbyEvent::StartLocate));
                    }),
                    nearby_command,
                ])
            }
            NearbyEvent::StartLocate => Command::new(|it| async move {
                nearby_service.start_locator_monitor(it.clone()).await;
            }),
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
