use crate::app::core::extensions::{CoreCommandContextUtils, CoreCommandUtils};
use crate::app::modules::AppModule;
use crate::app::operations::p2p::P2POperation;
use crate::app::view_models::peer::PeerViewModel;
use crate::app::{AppModel, BitBridge};
use crate::entities::device::DeviceInfo;
use crate::entities::finding_scope::FindingScope;
use crate::entities::peer::Peer;
use crate::entities::target::TransferTarget;
use crate::entities::transfer_session::TransferType;
use crate::app::transfer::module::TransferEvent;
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
    PeerUpdated { peer: Peer },
    PeerDisconnected { peer_id: String },
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
            NearbyEvent::PeerUpdated { peer } => {
                if let Some(existing_peer) = model.nearby.peers.iter_mut().find(|p| p.id == peer.id) {
                    *existing_peer = peer.clone();
                } else {
                    model.nearby.peers.push(peer.clone());
                }

                let mut peer_just_connected = false;
                let mut session_order_id = 0;
                let mut peer_lost_ownership = false;

                let owned_scopes = peer.owned_scopes();
                for session in model.transfer.sessions.iter_mut() {
                    if session.transfer_type != TransferType::Receive {
                        continue;
                    }

                    if let TransferTarget::P2P {
                        ref mut from_peer,
                        ref scope,
                        ..
                    } = session.target
                    {
                        let is_peer_owned = owned_scopes.iter().any(|s| s.scope_id() == scope);

                        if from_peer.is_none() && is_peer_owned {
                            *from_peer = Some(peer.clone());
                            peer_just_connected = true;
                            session_order_id = session.order_id;

                            break;
                        }
                        else if let Some(ref connected_peer) = from_peer {
                            if connected_peer.id == peer.id && !is_peer_owned {
                                *from_peer = None;
                                session.resources.clear();
                                session.progress.clear();
                                peer_lost_ownership = true;

                                break;
                            }
                        }
                    }
                }

                if peer_just_connected {
                    log::info!("Sending detail request for session {} to peer {}", session_order_id, peer.id);
                    return Command::event(crate::app::AppEvent::Transfer(TransferEvent::RequestSessionDetail {
                        peer_id: peer.id,
                        order_id: session_order_id,
                        password: None
                    })).then(Command::render());
                }

                if peer_lost_ownership {
                    return Command::render();
                }

                Command::render()
            }
            NearbyEvent::PeerDisconnected { peer_id } => {
                for session in model.transfer.sessions.iter_mut() {
                    if session.transfer_type != TransferType::Receive {
                        continue;
                    }

                    if let TransferTarget::P2P {
                        ref mut from_peer,
                        ..
                    } = session.target
                    {
                        if let Some(ref peer) = from_peer {
                            if peer.id == peer_id {
                                log::info!("Cleaning up session {} after peer disconnect", session.order_id);

                                *from_peer = None;

                                session.resources.clear();
                                session.progress.clear();

                                break;
                            }
                        }
                    }
                }

                Command::render()
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
