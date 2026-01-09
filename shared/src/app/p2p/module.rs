use crate::app::core::extensions::{CoreCommandContextUtils, CoreCommandUtils};
use crate::app::modules::AppModule;
use crate::app::operations::p2p::P2POperation;
use crate::app::transfer::module::TransferEvent;
use crate::app::view_models::peer::PeerViewModel;
use crate::app::{AppEvent, AppModel, BitBridge};
use crate::entities::device::DeviceInfo;
use crate::entities::finding_scope::FindingScope;
use crate::entities::peer::Peer;
use crate::entities::target::TransferTarget;
use crate::entities::transfer_session::TransferType;
use crux_core::{App, Command};
use schema::devlog::rpc_signalling::server::ScopeState;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct P2PModel {
    pub device: Option<DeviceInfo>,
    pub finding_scopes: Vec<FindingScope>,
    pub me: Option<Peer>,
    pub peers: Vec<Peer>
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct P2PViewModel {
    pub me: Option<PeerViewModel>,
    pub peers: Vec<PeerViewModel>
}

pub struct P2PModule;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum P2PEvent {
    Launch { auto_launch: bool },
    UpdateMe { new_peer: Peer },
    UpdateNearbyPeers { new_peer: Vec<Peer>, removed: Vec<Peer> },
    ClearNearbyPeers,
    AddFindingScope(FindingScope),
    RemoveFindingScope(FindingScope),
    PeerUpdated { peer: Peer },
    PeerDisconnected { peer_id: String },
    ScopeStateUpdated { scope_id: String, state: ScopeState, owner_id: Option<String> }
}

impl AppModule<BitBridge> for P2PModule {
    type Event = P2PEvent;
    type ViewModel = P2PViewModel;

    fn update(
        &self,
        event: Self::Event,
        model: &mut AppModel,
        _caps: &<BitBridge as App>::Capabilities
    ) -> Command<<BitBridge as App>::Effect, <BitBridge as App>::Event> {
        match event {
            P2PEvent::Launch { auto_launch } => {
                model.environment.auto_launch_nearby = auto_launch;
                Command::new(|it| async move {
                    it.app().start_nearby_server(auto_launch).await;
                })
                .then_render()
            }
            P2PEvent::UpdateNearbyPeers { new_peer, removed } => {
                model.p2p.peers.retain(|it| !removed.contains(it));
                model.p2p.peers.extend(new_peer);
                Command::render()
            }
            P2PEvent::ClearNearbyPeers => {
                model.p2p.peers.clear();
                Command::render()
            }
            P2PEvent::UpdateMe { new_peer } => {
                model.p2p.me = Some(new_peer);
                Command::render()
            }
            P2PEvent::AddFindingScope(scope) => {
                model.p2p.finding_scopes.retain(|s| s.scope_id() != scope.scope_id());
                model.p2p.finding_scopes.push(scope);
                let scopes = model.p2p.finding_scopes.clone();
                Command::handle_result(|it| async move { it.app().run(P2POperation::update_finding_scopes(scopes)).await })
            }
            P2PEvent::RemoveFindingScope(scope) => {
                model.p2p.finding_scopes.retain(|s| s != &scope);

                let scopes = model.p2p.finding_scopes.clone();
                Command::handle_result(|it| async move { it.app().run(P2POperation::update_finding_scopes(scopes)).await })
            }
            P2PEvent::PeerUpdated { mut peer } => {
                for scope in model.p2p.finding_scopes.iter() {
                    if scope.owner_peer_id() == Some(peer.id.as_str()) {
                        peer.add_scope(scope.clone());
                    }
                }

                if let Some(existing_peer) = model.p2p.peers.iter_mut().find(|p| p.id == peer.id) {
                    *existing_peer = peer.clone();
                } else {
                    model.p2p.peers.push(peer.clone());
                }

                let mut owner_just_connected = false;
                let mut session_order_id = 0;
                let owner_peer_id = peer.id.clone();

                for session in model.transfer.sessions.iter_mut() {
                    if session.transfer_type != TransferType::Receive {
                        continue;
                    }

                    if let TransferTarget::P2P { scope, ref from_peer, .. } = &session.target {
                        if from_peer.is_some() {
                            continue;
                        }

                        if scope.owner_peer_id() == Some(peer.id.as_str()) {
                            session.owner_connected(peer.clone());
                            let is_selected = model.transfer.selected_receive_session_id == Some(session.order_id);
                            if is_selected {
                                owner_just_connected = true;
                                session_order_id = session.order_id;
                            }
                        }
                    }
                }

                if owner_just_connected {
                    return Command::event(AppEvent::Transfer(TransferEvent::RequestSessionDetail {
                        peer_id: owner_peer_id,
                        order_id: session_order_id,
                        password: None
                    }))
                    .then(Command::render());
                }

                Command::render()
            }
            P2PEvent::PeerDisconnected { peer_id } => {
                for session in model.transfer.sessions.iter_mut() {
                    if session.transfer_type != TransferType::Receive {
                        continue;
                    }

                    if let TransferTarget::P2P { ref mut from_peer, .. } = session.target {
                        if let Some(ref peer) = from_peer {
                            if peer.id == peer_id {
                                log::info!("Cleaning up session {} after peer disconnect", session.order_id);
                                session.owner_disconnected();
                                break;
                            }
                        }
                    }
                }

                Command::render()
            }
            P2PEvent::ScopeStateUpdated { scope_id, state, owner_id } => {
                if let Some(scope) = model.p2p.finding_scopes.iter_mut().find(|s| s.scope_id() == scope_id) {
                    scope.update_state(state);
                    scope.set_owner_peer_id(owner_id.clone());
                }

                if let Some(oid) = &owner_id {
                    if let Some(peer) = model.p2p.peers.iter_mut().find(|p| p.id == *oid) {
                        if let Some(scope) = model.p2p.finding_scopes.iter().find(|s| s.scope_id() == scope_id) {
                            peer.update_scope(scope.clone());
                        }
                    }
                }

                for session in model.transfer.sessions.iter_mut() {
                    if session.transfer_type != TransferType::Receive {
                        continue;
                    }

                    if let TransferTarget::P2P { scope, ref mut from_peer, .. } = &mut session.target {
                        if scope.scope_id() != scope_id {
                            continue;
                        }

                        scope.update_state(state);
                        scope.set_owner_peer_id(owner_id.clone());

                        match (&owner_id, &from_peer) {
                            (Some(oid), None) => {
                                if let Some(peer) = model.p2p.peers.iter().find(|p| p.id == *oid) {
                                    session.owner_connected(peer.clone());
                                }
                            }
                            (None, Some(_)) => {
                                session.owner_disconnected();
                            }
                            (Some(new_oid), Some(old_peer)) if *new_oid != old_peer.id => {
                                session.owner_disconnected();
                                if let Some(peer) = model.p2p.peers.iter().find(|p| p.id == *new_oid) {
                                    session.owner_connected(peer.clone());
                                }
                            }
                            _ => {}
                        }
                    }
                }

                Command::render()
            }
        }
    }

    fn view(&self, model: &AppModel) -> Self::ViewModel {
        Self::ViewModel {
            me: model.p2p.me.as_ref().map(PeerViewModel::from),
            peers: model.p2p.peers.iter().map(PeerViewModel::from).collect()
        }
    }
}
