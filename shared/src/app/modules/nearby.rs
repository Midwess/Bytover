use crate::app::modules::AppModule;
use crate::app::nearby::finding_scope::FindingScope;
use crate::app::operations::device::GeoLocation;
use crate::app::operations::p2p::P2POperation;
use crate::app::operations::CoreOperation;
use crate::app::view_models::peer::PeerViewModel;
use crate::app::{AppEvent, AppModel, BitBridge};
use crate::di_container::DiContainer;
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

#[derive(Default)]
pub struct NearbyModule {}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, uniffi::Enum)]
pub enum NearbyEvent {
    Launch(),
    StartIpAddressMonitor,
    OnLocationUpdated(GeoLocation),
    OnIpAddressUpdated(String),

    UpdateMe { new: Peer },
    UpdateNearbyPeers { new: Vec<Peer>, removed: Vec<Peer> },

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
            NearbyEvent::Launch() => {
                let user = model.authentication.user.clone();
                let nearby_command = Command::new(|it| async move {
                    let nearby_service = DiContainer::get_instance().get_nearby_service();
                    nearby_service.start_service(user, it.clone()).await;
                });

                let start_ip_address_monitor_command = Command::new(|it| async move {
                    it.notify_shell(CoreOperation::Notified(AppEvent::Nearby(NearbyEvent::StartIpAddressMonitor)));
                });

                Command::all(vec![nearby_command, start_ip_address_monitor_command])
            }
            NearbyEvent::OnIpAddressUpdated(ip_address) => {
                let finding_scope = FindingScope::Local(ip_address.clone());
                model.nearby.finding_scopes.retain(|it| !it.is_local());
                model.nearby.finding_scopes.push(finding_scope);
                let finding_scopes = model.nearby.finding_scopes.clone();

                Command::new(|it| async move {
                    let _ = P2POperation::update_finding_scopes(finding_scopes).into_future(it.clone()).await;
                })
            }
            NearbyEvent::StartIpAddressMonitor => {
                Command::new(|it| async move {
                    let nearby_service = DiContainer::get_instance().get_nearby_service();
                    nearby_service.start_ip_address_monitor(it.clone()).await;
                })
            }
            NearbyEvent::OnLocationUpdated(location) => {
                let finding_scope = FindingScope::nearby_location(location);
                model.nearby.finding_scopes.retain(|it| !it.is_location());
                model.nearby.finding_scopes.extend(finding_scope);
                let finding_scopes = model.nearby.finding_scopes.clone();
                Command::new(|it| async move {
                    let _ = P2POperation::update_finding_scopes(finding_scopes).into_future(it).await;
                })
            }
            NearbyEvent::UpdateNearbyPeers { new, removed } => {
                model.nearby.peers.retain(|it| !removed.contains(it));
                model.nearby.peers.extend(new);
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
            NearbyEvent::UpdateMe { new } => {
                model.nearby.me = Some(new);
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
