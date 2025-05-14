use crate::app::file_system::file::{LocalResource, ResourceType};
use crate::app::modules::AppModule;
use crate::app::operations::CoreOperation;
use crate::app::transfer::file_selection_service::ResourceSelection;
use crate::app::transfer::session::{TransferSession, TransferType};
use crate::app::transfer::target::TransferTarget;
use crate::app::transfer::transfer_selection::TransferMethodSelection;
use crate::app::view_models::avatar::AvatarViewModel;
use crate::app::view_models::peer::PeerViewModel;
use crate::app::view_models::receive_session::{FileReceiveResourceViewModel, ImageReceiveResourceViewModel, ReceiveSessionViewModel, VideoReceiveResourceViewModel};
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
    nearby_peers: Vec<PeerViewModel>,
    received_sessions: Vec<ReceiveSessionViewModel>
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
    TransferCanceled {
        session_id: u64
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
        removed: Vec<LocalResource>,
        updated: Vec<LocalResource>
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
            TransferEvent::UpdateResourcesModel { new, removed, updated } => {
                for new in new {
                    if model.transfer.selected_resources.iter().any(|it| it.order_id == new.order_id) {
                        continue;
                    }

                    model.transfer.selected_resources.push(new);
                }

                model
                    .transfer
                    .selected_resources
                    .retain(|it| !removed.iter().any(|removed| removed.order_id == it.order_id));

                for updated in updated {
                    if let Some(index) = model.transfer.selected_resources.iter().position(|it| it.order_id == updated.order_id) {
                        model.transfer.selected_resources[index] = updated;
                    }
                }

                Command::done()
            }
            TransferEvent::TransferCanceled { session_id, .. } => {
                let Some(session) = model.transfer.transfer_sessions.iter_mut().find(|it| it.order_id == session_id) else {
                    return Command::done();
                };

                session.cancel();

                let session = session.clone();
                Command::new(|it| async move {
                    let transfer_service = DiContainer::get_instance().get_transfer_service();
                    transfer_service.cancel_transfer(session, it.clone()).await;
                })
            }
            TransferEvent::StartTransfer { target_id } => {
                let selected_resources = model.transfer.selected_resources.clone();
                let transfer_targets = model.transfer.transfer_targets.clone();
                let Some(target) = transfer_targets.iter().find(|it| it.id() == target_id).cloned() else {
                    return Command::done();
                };

                let duplicated_session = model
                    .transfer
                    .transfer_sessions
                    .iter()
                    .filter(|it| it.transfer_type == TransferType::Send)
                    .find(|it| {
                        if it.is_completed() {
                            return false;
                        }

                        if let TransferTarget::Nearby(peer) = &target {
                            it.peer_id() == Some(peer.id()) && it.transfer_type == TransferType::Send
                        } else {
                            false
                        }
                    })
                    .cloned();

                Command::new(async |it| {
                    let transfer_service = DiContainer::get_instance().get_transfer_service();
                    if let Some(duplicated_session) = duplicated_session {
                        transfer_service.cancel_transfer(duplicated_session.clone(), it.clone()).await;
                        return;
                    }

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
                for new in new {
                    if model.transfer.transfer_sessions.iter().any(|it| it.order_id == new.order_id) {
                        continue;
                    }

                    model.transfer.transfer_sessions.push(new);
                }

                model
                    .transfer
                    .transfer_sessions
                    .retain(|it| !removed.iter().any(|removed| removed.order_id == it.order_id));

                for updated in updated {
                    if let Some(index) = model.transfer.transfer_sessions.iter().position(|it| it.order_id == updated.order_id) {
                        model.transfer.transfer_sessions[index] = updated;
                    }
                }

                Command::new(|it| async move {
                    it.notify_shell(CoreOperation::Render);
                })
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
            received_sessions: model.transfer.transfer_sessions
                .iter()
                .filter(|it| it.transfer_type == TransferType::Receive)
                .filter_map(|it| {
                    let Some(peer) = model.transfer.transfer_targets
                        .iter()
                        .filter_map(|target| match target {
                            TransferTarget::Nearby(peer) => Some(peer),
                            _ => None
                        })
                        .find(|peer| peer.id() == it.peer_id().unwrap_or_default())
                        .cloned()
                    else {
                        return None;
                    };

                    let image_resources = it.resources.iter().filter_map(|resource| {
                        if resource.r#type != ResourceType::Image {
                            return None;
                        }

                        let Some(progress) = it.progress.iter().find(|it| it.resource_order_id == resource.order_id) else {
                            return None;
                        };

                        Some(ImageReceiveResourceViewModel {
                            model: SelectedResourceViewModel::from(resource),
                            is_completed: progress.status.is_completed(),
                        })
                    }).collect();

                    let video_resources = it.resources.iter().filter_map(|resource| {
                        if resource.r#type != ResourceType::Video {
                            return None;
                        }

                        let Some(progress) = it.progress.iter().find(|it| it.resource_order_id == resource.order_id) else {
                            return None;
                        };

                        Some(VideoReceiveResourceViewModel {
                            model: SelectedResourceViewModel::from(resource),
                            is_completed: progress.status.is_completed(),
                        })
                    }).collect();

                    let file_resources = it.resources.iter().filter_map(|resource| {
                        if resource.r#type != ResourceType::File {
                            return None;
                        }

                        let Some(progress) = it.progress.iter().find(|it| it.resource_order_id == resource.order_id) else {
                            return None;
                        };

                        Some(FileReceiveResourceViewModel {
                            model: SelectedResourceViewModel::from(resource),
                            is_completed: progress.status.is_completed(),
                        })
                    }).collect();

                    Some(ReceiveSessionViewModel {
                        id: it.order_id,
                        peer_avatar: AvatarViewModel::new(peer.avatar_url.clone()),
                        peer_name: peer.name.clone().unwrap_or(peer.device.name.clone()),
                        peer_description: "Nearby".to_owned(),
                        is_completed: it.is_completed(),
                        is_in_progress: !it.is_completed(),
                        display_download_speed: format!("{:.2} MB/s", it.speed(1000)),
                        progress: it.total_progress(),
                        image_resources,
                        video_resources,
                        file_resources,
                    })
                }).collect(),
            nearby_peers: model
                .transfer
                .transfer_targets
                .iter()
                .filter_map(|it| match it {
                    TransferTarget::Nearby(peer) => {
                        let send_session = model.transfer.transfer_sessions.iter().find(|it| {
                            it.transfer_type == TransferType::Send && it.peer_id().as_ref().unwrap().to_string() == peer.id
                        });

                        Some(PeerViewModel {
                            id: peer.id.clone(),
                            display_name: peer.name.clone().unwrap_or(peer.device.name.clone()),
                            avatar: AvatarViewModel::new(peer.avatar_url.clone()),
                            device: peer.device.clone(),
                            transfer_progress: send_session.map(|it| it.total_progress()).unwrap_or(0.0),
                            display_upload_speed: send_session.map(|it| it.status().to_string()),
                            display_download_speed: None // The download speed is displayed in the received screen
                        })
                    }
                    _ => None
                })
                .collect()
        }
    }
}
