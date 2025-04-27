use crate::app::file_system::file::LocalResource;
use crate::app::modules::AppModule;
use crate::app::operations::CoreOperation;
use crate::app::transfer::file_selection_service::ResourceSelection;
use crate::app::transfer::session::{TransferSession, TransferType};
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
    is_loading_selected_resources: bool,
    transfer_method_selection: TransferMethodSelection,
    transfer_sessions: Vec<TransferSession>,
    transfer_targets: Vec<TransferTarget>
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct TransferViewModel {
    selected_resources: Vec<SelectedResourceViewModel>,
    is_loading_selected_resources: bool,
    transfer_method_selection: TransferMethodSelection,
    nearby_peers: Vec<PeerViewModel>
}

#[derive(Default)]
pub struct TransferModule {}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, uniffi::Enum)]
pub enum TransferEvent {
    // Event from shell
    Launch(),
    // This event is used to notify the core that the shell need sometime to load resources
    // The core will control the loading progress after the AddResources is triggered
    BeginLoadingResources(),
    EndLoadingResources(),
    AddResources(Vec<ResourceSelection>),
    RemoveResource(u64),
    StartTransfer {
        target_id: String
    },
    TransferRequest {
        request_id: String,
        remote_session: TransferSessionMessage,
        peer: Peer
    },
    // Event from core
    UpdateTransferSessions {
        new: Vec<TransferSession>,
        removed: Vec<TransferSession>,
        updated: Vec<TransferSession>
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
            TransferEvent::Launch() => {
                let resource_transfer_selection_service = DiContainer::get_instance().get_resource_transfer_selection_service();
                Command::new(|it| async move {
                    resource_transfer_selection_service.load_resources(it).await;
                })
            }
            TransferEvent::BeginLoadingResources() => {
                model.transfer.is_loading_selected_resources = true;
                Command::new(async |it| {
                    it.notify_shell(CoreOperation::Render);
                })
            }
            TransferEvent::EndLoadingResources() => {
                model.transfer.is_loading_selected_resources = false;
                Command::new(|it| async move {
                    it.notify_shell(CoreOperation::Render);
                })
            }
            TransferEvent::AddResources(selections) => Command::new(|it| async move {
                let resource_transfer_selection_service = DiContainer::get_instance().get_resource_transfer_selection_service();
                resource_transfer_selection_service.add_resources(it.clone(), selections).await;
                it.notify_shell(CoreOperation::Render);
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
            TransferEvent::StartTransfer { target_id } => {
                let selected_resources = model.transfer.selected_resources.clone();
                let transfer_targets = model.transfer.transfer_targets.clone();
                let Some(target) = transfer_targets.iter().find(|it| it.id() == target_id).cloned() else {
                    return Command::done();
                };

                Command::new(async |it| {
                    let transfer_service = DiContainer::get_instance().get_transfer_service();
                    transfer_service.transfer(selected_resources, target, it).await;
                })
            }
            TransferEvent::TransferRequest {
                request_id,
                remote_session,
                peer
            } => {
                let transfer_service = DiContainer::get_instance().get_transfer_service();
                Command::new(|it| async move {
                    transfer_service.received_session_request((request_id, remote_session), peer, it).await;
                    log::info!(target: "transfer", "Done download, shell should done");
                })
            }
            TransferEvent::UpdateTransferSessions { new, removed, updated } => {
                model.transfer.transfer_sessions.extend(new);
                model
                    .transfer
                    .transfer_sessions
                    .retain(|it| !removed.iter().any(|removed| removed.order_id == it.order_id));

                for updated in updated {
                    if let Some(index) = model.transfer.transfer_sessions.iter().position(|it| it.order_id == updated.order_id) {
                        model.transfer.transfer_sessions[index] = updated;
                    }
                }

                Command::done()
            }
            TransferEvent::UpdateTransferTargets { new, removed } => {
                model.transfer.transfer_targets.extend(new);
                model.transfer.transfer_targets.retain(|it| !removed.iter().any(|removed| removed.id() == it.id()));
                Command::done()
            }
        }
    }

    fn view(&self, model: &AppModel) -> Self::ViewModel {
        Self::ViewModel {
            is_loading_selected_resources: model.transfer.is_loading_selected_resources,
            selected_resources: model.transfer.selected_resources.iter().map(SelectedResourceViewModel::from).collect(),
            transfer_method_selection: model.transfer.transfer_method_selection.clone(),
            nearby_peers: model
                .transfer
                .transfer_targets
                .iter()
                .filter_map(|it| match it {
                    TransferTarget::Nearby(peer) => {
                        let session = model
                            .transfer
                            .transfer_sessions
                            .iter()
                            .find(|it| it.peer_id().as_ref().unwrap().to_string() == peer.id);

                        Some(PeerViewModel {
                            id: peer.id.clone(),
                            display_name: peer.name.clone().unwrap_or(peer.device.name.clone()),
                            avatar: AvatarViewModel::new(peer.avatar_url.clone()),
                            device: peer.device.clone(),
                            transfer_progress: session.map(|it| it.total_progress()).unwrap_or(0.0),
                            display_upload_speed: session.and_then(|it| {
                                if it.transfer_type == TransferType::Send {
                                    if it.is_initializing() {
                                        return Some("Initializing".to_owned());
                                    }
                                    let bytes_per_second = it.bytes_per_second();
                                    Some(format!("{:.2} MB/s", bytes_per_second as f64 / 1024.0 / 1024.0))
                                } else {
                                    None
                                }
                            }),
                            display_download_speed: session.and_then(|it| {
                                if it.transfer_type == TransferType::Receive {
                                    if it.is_initializing() {
                                        return Some("Initializing".to_owned());
                                    }

                                    let bytes_per_second = it.bytes_per_second();
                                    Some(format!("{:.2} MB/s", bytes_per_second as f64 / 1024.0 / 1024.0))
                                } else {
                                    None
                                }
                            })
                        })
                    }
                    _ => None
                })
                .collect()
        }
    }
}
