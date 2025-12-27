use crate::app::core::extensions::{CoreCommandContextUtils, CoreCommandUtils};
use crate::app::modules::AppModule;
use crate::app::operations::p2p::P2POperation;
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
    Launch { auto_launch: bool },
    UpdateMe { new_peer: Peer },
    UpdateNearbyPeers { new_peer: Vec<Peer>, removed: Vec<Peer> },
    ClearNearbyPeers,
    AddFindingScope(FindingScope),
    RemoveFindingScope(FindingScope),
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
            NearbyEvent::Launch {auto_launch} => {
                model.environment.auto_launch_nearby = auto_launch;
                Command::new(|it| async move {
                    it.app().start_nearby_server(auto_launch).await;
                }).then_render()
            }
            NearbyEvent::UpdateNearbyPeers { new_peer, removed } => {
                model.nearby.peers.retain(|it| !removed.contains(it));
                model.nearby.peers.extend(new_peer);
                Command::render()
            }
            NearbyEvent::ClearNearbyPeers => {
                model.nearby.peers.clear();
                Command::render()
            }
            NearbyEvent::UpdateMe { new_peer } => {
                model.nearby.me = Some(new_peer);
                Command::render()
            }
            NearbyEvent::AddFindingScope(scope) => {
                if !model.nearby.finding_scopes.contains(&scope) {
                    model.nearby.finding_scopes.push(scope);

                    let scopes = model.nearby.finding_scopes.clone();
                    return Command::handle_result(|it| async move {
                        it.app().run(P2POperation::update_finding_scopes(scopes)).await
                    });
                }
                Command::done()
            }
            NearbyEvent::RemoveFindingScope(scope) => {
                model.nearby.finding_scopes.retain(|s| s != &scope);

                let scopes = model.nearby.finding_scopes.clone();
                Command::handle_result(|it| async move {
                    it.app().run(P2POperation::update_finding_scopes(scopes)).await
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
