use crate::app::modules::AppModule;
use crate::app::operations::device::GeoLocation;
use crate::app::operations::p2p::P2POperation;
use crate::app::operations::CoreOperation;
use crate::app::nearby::finding_scope::FindingScope;
use crate::app::view_models::peer::PeerViewModel;
use crate::app::{AppModel, BitBridge};
use crate::di_container::DiContainer;
use crate::entities::device::DeviceInfo;
use crate::entities::peer::Peer;
use crux_core::{App, Command};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct NearbyModel {
    pub device: Option<DeviceInfo>,
    pub finding_scopes: Vec<FindingScope>,
    pub peers: Vec<Peer>
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct NearbyViewModel {
    pub peers: Vec<PeerViewModel>
}

#[derive(Default)]
pub struct NearbyModule {

}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, uniffi::Enum)]
pub enum NearbyEvent {
    Launch(),
    OnLocationUpdated(GeoLocation),
    OnIpAddressUpdated(String),

    UpdateNearbyPeers {
        new: Vec<Peer>,
        removed: Vec<Peer>
    },

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
                Command::new(|it| async move {
                    let nearby_service = DiContainer::get_instance().get_nearby_service();
                    nearby_service.start_service(user, it.clone()).await;
                })
            },
            NearbyEvent::OnIpAddressUpdated(ip_address) => {
                log::info!(target: "nearby", "Updated ip address {}", ip_address);
                let finding_scope = FindingScope::Local(ip_address.clone());
                model.nearby.finding_scopes.retain(|it| !it.is_local());
                model.nearby.finding_scopes.push(finding_scope);
                let finding_scopes = model.nearby.finding_scopes.clone();

                Command::new(|it| async move {
                    let _ = P2POperation::update_finding_scopes(finding_scopes).into_future(it.clone()).await;
                })
            },
            NearbyEvent::OnLocationUpdated(location) => {
                let finding_scope = FindingScope::nearby_location(location);
                model.nearby.finding_scopes.retain(|it| !it.is_location());
                model.nearby.finding_scopes.extend(finding_scope);
                let finding_scopes = model.nearby.finding_scopes.clone();
                Command::new(|it| async move {
                    let _ = P2POperation::update_finding_scopes(finding_scopes).into_future(it).await;
                })
            },
            NearbyEvent::UpdateNearbyPeers {
                new,
                removed
            } => {
                model.nearby.peers.retain(|it| !removed.contains(it));
                model.nearby.peers.extend(new);
                Command::new(|it| async move {
                    it.notify_shell(CoreOperation::Render);
                })
            },
            NearbyEvent::ClearNearbyPeers => {
                model.nearby.peers.clear();
                Command::new(|it| async move {
                    it.notify_shell(CoreOperation::Render);
                })
            }
        }
    }

    fn view(&self, model: &AppModel) -> Self::ViewModel {
        Self::ViewModel {
            peers: model.nearby.peers.iter().map(PeerViewModel::from).collect()
        }
    }
}
