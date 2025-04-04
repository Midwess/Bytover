use crate::app::file_system::file::LocalResource;
use crate::app::modules::AppModule;
use crate::app::operations::device::GeoLocation;
use crate::app::operations::transfer::TransferOperation;
use crate::app::transfer::file_selection_service::ResourceSelection;
use crate::app::transfer::finding_scope::FindingScope;
use crate::app::transfer::transfer_selection::TransferMethodSelection;
use crate::app::view_models::avatar::AvatarViewModel;
use crate::app::view_models::peer::PeerViewModel;
use crate::app::view_models::selected_resource::SelectedResourceViewModel;
use crate::app::{AppModel, BitBridge};
use crate::di_container::DiContainer;
use crate::entities::peer::Peer;
use crux_core::{App, Command};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TransferModel {
    selected_resources: Vec<LocalResource>,
    transfer_method_selection: TransferMethodSelection,
    finding_scopes: Vec<FindingScope>,
    peers: Vec<Peer>
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct TransferViewModel {
    selected_resources: Vec<SelectedResourceViewModel>,
    transfer_method_selection: TransferMethodSelection,
    peers: Vec<PeerViewModel>
}

#[derive(Default)]
pub struct TransferModule {}

#[derive(Clone, Debug, Serialize, Deserialize, uniffi::Enum)]
pub enum TransferEvent {
    // Event from shell
    Launched(),
    AddResources(Vec<ResourceSelection>),
    RemoveResource(u64),
    OnLocationUpdated(GeoLocation),
    OnIpAddressUpdated(String),
    OnNewPeer(Peer),
    OnPeerLeaved(Peer),

    // Event from core
    UpdateResourcesModel {
        new: Vec<LocalResource>,
        removed: Vec<LocalResource>
    },
    UpdatePeers {
        new: Vec<Peer>,
        removed: Vec<Peer>
    }
}

impl AppModule<BitBridge> for TransferModule {
    type Event = TransferEvent;
    type ViewModel = TransferViewModel;

    fn update(
        &self,
        event: Self::Event,
        model: &mut AppModel,
        _caps: &<BitBridge as App>::Capabilities
    ) -> Command<<BitBridge as App>::Effect, <BitBridge as App>::Event> {
        match event {
            TransferEvent::Launched() => {
                let user = model.authentication.user.clone();
                Command::new(|it| async move {
                    let resource_transfer_selection_service = DiContainer::get_instance().get_resource_transfer_selection_service();
                    resource_transfer_selection_service.load_resources(it.clone()).await;
                    let nearby_service = DiContainer::get_instance().get_nearby_service();
                    nearby_service.init(user, it.clone()).await;
                })
            }
            TransferEvent::AddResources(selections) => Command::new(|it| async move {
                let resource_transfer_selection_service = DiContainer::get_instance().get_resource_transfer_selection_service();
                resource_transfer_selection_service.add_resources(it, selections).await;
            }),
            TransferEvent::RemoveResource(id) => Command::new(|it| async move {
                let resource_transfer_selection_service = DiContainer::get_instance().get_resource_transfer_selection_service();
                resource_transfer_selection_service.remove_resource(it, id).await;
            }),
            TransferEvent::UpdateResourcesModel { new, removed } => {
                model.transfer.selected_resources.extend(new);
                model
                    .transfer
                    .selected_resources
                    .retain(|it| !removed.iter().any(|removed| removed.order_id == it.order_id));

                Command::done()
            }
            TransferEvent::OnIpAddressUpdated(ip_address) => {
                let finding_scope = FindingScope::Local(ip_address.clone());
                model.transfer.finding_scopes.retain(|it| !it.is_local());
                model.transfer.finding_scopes.push(finding_scope);
                let finding_scopes = model.transfer.finding_scopes.clone();

                Command::new(|it| async move {
                    TransferOperation::update_finding_scopes(finding_scopes).into_future(it.clone()).await;
                })
            }
            TransferEvent::OnLocationUpdated(location) => {
                let finding_scope = FindingScope::nearby_location(location);
                model.transfer.finding_scopes.retain(|it| !it.is_location());
                model.transfer.finding_scopes.extend(finding_scope);
                let finding_scopes = model.transfer.finding_scopes.clone();
                Command::new(|it| async move {
                    TransferOperation::update_finding_scopes(finding_scopes).into_future(it).await;
                })
            }
            TransferEvent::UpdatePeers { new, removed } => {
                model.transfer.peers.extend(new);
                model.transfer.peers.retain(|it| !removed.iter().any(|removed| removed.id == it.id));
                Command::done()
            }
            TransferEvent::OnNewPeer(peer) => Command::new(async |it| {
                let nearby_service = DiContainer::get_instance().get_nearby_service();
                nearby_service.on_new_peer_come(peer, it).await;
            }),
            TransferEvent::OnPeerLeaved(peer) => Command::new(async |it| {
                let nearby_service = DiContainer::get_instance().get_nearby_service();
                nearby_service.on_peer_leaved(peer, it).await;
            })
        }
    }

    fn view(&self, model: &AppModel) -> Self::ViewModel {
        Self::ViewModel {
            selected_resources: model.transfer.selected_resources.iter().map(SelectedResourceViewModel::from).collect(),
            transfer_method_selection: model.transfer.transfer_method_selection.clone(),
            peers: model
                .transfer
                .peers
                .iter()
                .map(|it| PeerViewModel {
                    id: it.id.clone(),
                    display_name: it.name.clone().unwrap_or(it.device.name.clone()),
                    avatar: AvatarViewModel::new(it.avatar_url.clone()),
                    device: it.device.clone()
                })
                .collect()
        }
    }
}
