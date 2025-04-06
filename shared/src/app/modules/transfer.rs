use crate::app::file_system::file::LocalResource;
use crate::app::modules::AppModule;
use crate::app::operations::device::GeoLocation;
use crate::app::operations::transfer::TransferOperation;
use crate::app::operations::CoreOperation;
use crate::app::transfer::file_selection_service::ResourceSelection;
use crate::app::transfer::finding_scope::FindingScope;
use crate::app::transfer::session::{TransferProgress, TransferSession};
use crate::app::transfer::target::TransferTarget;
use crate::app::transfer::transfer_selection::TransferMethodSelection;
use crate::app::view_models::avatar::AvatarViewModel;
use crate::app::view_models::peer::PeerViewModel;
use crate::app::view_models::selected_resource::SelectedResourceViewModel;
use crate::app::{AppModel, BitBridge};
use crate::di_container::DiContainer;
use crate::entities::peer::Peer;
use crux_core::{App, Command};
use schema::devlog::bitbridge::TransferSessionMessage;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TransferModel {
    selected_resources: Vec<LocalResource>,
    transfer_method_selection: TransferMethodSelection,
    finding_scopes: Vec<FindingScope>,
    transfer_targets: Vec<TransferTarget>,
    transfer_sessions: Vec<TransferSession>
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct TransferViewModel {
    selected_resources: Vec<SelectedResourceViewModel>,
    transfer_method_selection: TransferMethodSelection,
    nearby_peers: Vec<PeerViewModel>
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
    StartTransfer {
        target_id: String
    },
    TransferRequest(TransferSessionMessage, Peer),
    NewTransferProgress {
        session_id: u64,
        progress: TransferProgress
    },

    // Event from core
    UpdateTransferSessions {
        new: Vec<TransferSession>,
        removed: Vec<TransferSession>
    },
    UpdateResourcesModel {
        new: Vec<LocalResource>,
        removed: Vec<LocalResource>
    },
    UpdateTransferTargets {
        new: Vec<TransferTarget>,
        removed: Vec<TransferTarget>
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
            TransferEvent::UpdateTransferTargets { new, removed } => {
                model.transfer.transfer_targets.extend(new);
                model.transfer.transfer_targets.retain(|it| !removed.iter().any(|removed| removed == it));
                Command::done()
            }
            TransferEvent::OnNewPeer(peer) => Command::new(async |it| {
                let nearby_service = DiContainer::get_instance().get_nearby_service();
                nearby_service.on_new_nearby_peer_come(peer, it).await;
            }),
            TransferEvent::OnPeerLeaved(peer) => Command::new(async |it| {
                let nearby_service = DiContainer::get_instance().get_nearby_service();
                nearby_service.on_nearby_peer_leaved(peer, it).await;
            }),
            TransferEvent::StartTransfer { target_id } => {
                let selected_resources = model.transfer.selected_resources.clone();
                let transfer_targets = model.transfer.transfer_targets.clone();
                Command::new(async |it| {
                    let transfer_service = DiContainer::get_instance().get_transfer_service();
                    transfer_service.transfer(selected_resources, transfer_targets, target_id, it).await;
                })
            }
            TransferEvent::TransferRequest(request, peer) => {
                let transfer_service = DiContainer::get_instance().get_transfer_service();
                Command::new(|it| async move {
                    transfer_service.received_session_request(request, peer, it).await;
                })
            }
            TransferEvent::NewTransferProgress { session_id, progress } => {
                let Some(session) = model.transfer.transfer_sessions.iter_mut().find(|it| it.order_id == session_id) else {
                    return Command::done();
                };

                session.update_progress(progress);
                Command::new(|it| async move {
                    it.request_from_shell(CoreOperation::Render).await;
                })
            }
            TransferEvent::UpdateTransferSessions { new, removed } => {
                model.transfer.transfer_sessions.extend(new);
                model
                    .transfer
                    .transfer_sessions
                    .retain(|it| !removed.iter().any(|removed| removed.order_id == it.order_id));
                Command::done()
            }
        }
    }

    fn view(&self, model: &AppModel) -> Self::ViewModel {
        Self::ViewModel {
            selected_resources: model.transfer.selected_resources.iter().map(SelectedResourceViewModel::from).collect(),
            transfer_method_selection: model.transfer.transfer_method_selection.clone(),
            nearby_peers: model
                .transfer
                .transfer_targets
                .iter()
                .filter_map(|it| match it {
                    TransferTarget::Nearby(peer) => Some(PeerViewModel {
                        id: peer.id.clone(),
                        display_name: peer.name.clone().unwrap_or(peer.device.name.clone()),
                        avatar: AvatarViewModel::new(peer.avatar_url.clone()),
                        device: peer.device.clone()
                    })
                })
                .collect()
        }
    }
}
