use crate::app::core_utils::CoreCommandUtils;
use crate::app::file_system::file::{LocalResource, LocalResourcePath, ResourceType};
use crate::app::modules::AppModule;
use crate::app::operations::device::OpenOperation;
use crate::app::operations::dialog::{AlertDialog, DialogOperation};
use crate::app::operations::persistent::{LocalResourcePersistentOperation, TransferSessionPersistentOperation};
use crate::app::operations::CoreOperation;
use crate::app::transfer::file_selection_service::{ResourceSelection, ResourceTransferSelectionService};
use crate::app::transfer::session::{TransferProgress, TransferSession, TransferStatus, TransferType};
use crate::app::transfer::target::TransferTarget;
use crate::app::transfer::transfer_selection::TransferMethodSelection;
use crate::app::transfer::transfer_service::TransferService;
use crate::app::view_models::avatar::AvatarViewModel;
use crate::app::view_models::cloud_session::CloudSession;
use crate::app::view_models::peer::PeerViewModel;
use crate::app::view_models::receive_session::{
    FileReceiveResourceViewModel,
    ImageReceiveResourceViewModel,
    ReceiveCloudSessionViewModel,
    ReceiveSessionViewModel,
    VideoReceiveResourceViewModel
};
use crate::app::view_models::selected_resource::SelectedResourceViewModel;
use crate::app::{AppEvent, AppModel, BitBridge};
use crate::entities::peer::Peer;
use crux_core::{App, Command};
use devlog_sdk::distributed_id::id_to_datetime;
use schema::devlog::bitbridge::TransferSessionMessage;
use serde::{Deserialize, Serialize};
use url::Url;

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
    received_sessions: Vec<ReceiveSessionViewModel>,
    received_cloud_sessions: Vec<ReceiveCloudSessionViewModel>,
    cloud_session: Option<CloudSession>
}

pub struct TransferModule {
    pub resource_selection_service: &'static ResourceTransferSelectionService,
    pub transfer_service: &'static TransferService
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum TransferEvent {
    // Event from shell
    Launch(),
    // This event is used to notify the core that the shell need sometime to load resources
    // The core will control the loading progress after the AddResources is triggered
    BeginLoadingResources(),
    EndLoadingResources(),
    AddResources(Vec<ResourceSelection>),
    RemoveResource(u64),
    OpenSession {
        session_id: u64
    },
    DeleteSession {
        session_id: u64
    },
    StartPublicTransfer {
        password: Option<String>
    },
    StartTransfer {
        target_id: String
    },
    CancelTransfer {
        session_id: u64,
        transfer_type: TransferType
    },
    TransferCanceled {
        session_id: u64
    },
    TransferRequest {
        remote_session: TransferSessionMessage,
        peer: Peer
    },
    // Event from core
    UpdateTransferSessions {
        // Loaded from our database
        loaded: Vec<TransferSession>,
        // New sessions
        added: Vec<TransferSession>,
        // Removed sessions
        removed: Vec<(u64, TransferType)>,
        // Updated sessions
        updated: Vec<TransferSession>
    },
    UpdateResourcesModel {
        loaded: Vec<LocalResource>,
        added: Vec<LocalResource>,
        removed: Vec<LocalResource>,
        updated: Vec<LocalResource>
    },
    UpdateTransferTargets {
        added: Vec<TransferTarget>,
        removed: Vec<TransferTarget>
    },
    OpenSessionResource {
        session_id: u64,
        resource_id: u64
    },
    OpenSelectedResource {
        resource_id: u64
    },
    SessionResourceThumbnailFullFilled {
        session_id: u64,
        resource_id: u64,
        path: LocalResourcePath
    },
    UpdateResourceTransferProgresses {
        session_id: u64,
        progresses: Vec<TransferProgress>
    },
    FindPublicSession {
        keywords: String
    },
    ViewPublicSession {
        password: Option<String>,
        session_id: u64,
        transfer_type: TransferType
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
        let transfer_service = self.transfer_service;
        let resource_selection_service = self.resource_selection_service;
        match event {
            TransferEvent::Launch() => Command::new(|it| async move {
                resource_selection_service.load_resources(it.clone()).await;
                transfer_service.load_transfer_sessions(it).await;
            }),
            TransferEvent::BeginLoadingResources() => {
                model.transfer.is_loading_selected_resources = true;
                Command::new(|it| async move {
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
                resource_selection_service.add_resources(it.clone(), selections).await;
                it.notify_shell(CoreOperation::Render);
            }),
            TransferEvent::RemoveResource(id) => {
                model.transfer.selected_resources.retain(|it| it.order_id != id);
                Command::new(|it| async move {
                    it.notify_shell(CoreOperation::Render);
                })
            }
            TransferEvent::CancelTransfer { session_id, transfer_type } => {
                let Some(session) = model
                    .transfer
                    .transfer_sessions
                    .iter()
                    .find(|it| it.order_id == session_id && it.transfer_type.eq(&transfer_type))
                    .cloned()
                else {
                    return Command::new(|it| async move {
                        DialogOperation::toast("Session not found".to_string()).into_future(it.clone()).await;
                    });
                };

                if session.is_completed() && session.target.is_peer() {
                    return Command::new(|it| async move {
                        DialogOperation::toast("Session is already completed".to_string()).into_future(it.clone()).await;
                    });
                }

                Command::new(|it| async move {
                    if !session.is_completed() {
                        let confirmation = DialogOperation::alert(AlertDialog::confirmation(
                            "Cancel the transfer ?".to_string(),
                            "Yes".to_string(),
                            Some("No".to_string())
                        ))
                        .into_future(it.clone())
                        .await;

                        if !confirmation {
                            return;
                        }
                    }

                    transfer_service.delete_session(session, it.clone()).await;
                })
            }
            TransferEvent::DeleteSession { session_id } => {
                let Some(session) = model.transfer.transfer_sessions.iter().find(|it| it.order_id == session_id).cloned() else {
                    return Command::done();
                };

                if !session.is_completed() {
                    return Command::new(|it| async move {
                        DialogOperation::toast("Session is still in progress".to_string()).into_future(it.clone()).await;
                    });
                }

                Command::new(|it| async move {
                    transfer_service.delete_session(session, it.clone()).await;
                })
            }
            TransferEvent::UpdateResourcesModel {
                loaded,
                added: new,
                removed,
                updated
            } => {
                let mut command = Command::empty();

                for loaded in loaded {
                    if model.transfer.selected_resources.iter().any(|it| it.order_id == loaded.order_id) {
                        continue;
                    }

                    model.transfer.selected_resources.push(loaded);
                }

                for new in new.iter() {
                    if model.transfer.selected_resources.iter().any(|it| it.order_id == new.order_id) {
                        continue;
                    }

                    model.transfer.selected_resources.push(new.clone());
                }

                if !new.is_empty() {
                    let new = new.clone();
                    command = command.and(Command::new(|it| async move {
                        LocalResourcePersistentOperation::add(new).into_future(it.clone()).await;
                    }));
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

                model.transfer.selected_resources.sort_by(|a, b| b.order_id.cmp(&a.order_id));

                command.then_render()
            }
            TransferEvent::TransferCanceled { session_id, .. } => {
                let Some(session) = model.transfer.transfer_sessions.iter_mut().find(|it| it.order_id == session_id) else {
                    return Command::done();
                };

                session.cancel();

                let session = session.clone();
                Command::new(|it| async move {
                    transfer_service.delete_session(session, it.clone()).await;
                })
            }
            TransferEvent::StartPublicTransfer { password } => {
                let selected_resources = model.transfer.selected_resources.clone();
                if let Some(user) = model.authentication.user.clone() {
                    return Command::new(|it| async move {
                        transfer_service
                            .transfer(
                                user.clone(),
                                selected_resources,
                                TransferTarget::Internet {
                                    is_required_password: password.is_some(),
                                    password,
                                    access_url: None,
                                    from_user: user
                                },
                                it
                            )
                            .await;
                    })
                }

                Command::new(|it| async move {
                    it.notify_shell(CoreOperation::Dialog(DialogOperation::Toast("Unauthenticated".to_owned())));
                })
            }
            TransferEvent::StartTransfer { target_id } => {
                log::info!("Start transferring to target {target_id:?}");
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
                    .find(|it| it.peer_id().map(|id| id.to_string()) == Some(target.id()))
                    .cloned();

                let Some(user) = model.authentication.user.clone() else {
                    return Command::new(|it| async move {
                        DialogOperation::toast("unauthenticated".to_owned()).into_future(it).await;
                    })
                };

                Command::new(|it| async move {
                    if let Some(duplicated_session) = duplicated_session {
                        it.send_event(AppEvent::Transfer(TransferEvent::CancelTransfer {
                            session_id: duplicated_session.order_id,
                            transfer_type: duplicated_session.transfer_type
                        }));
                        return;
                    }

                    transfer_service.transfer(user, selected_resources, target, it).await;
                })
            }
            TransferEvent::TransferRequest { remote_session, peer } => Command::new(|it| async move {
                transfer_service.received_session_request(remote_session, peer, it).await;
                log::info!(target: "transfer", "Done download, shell should done");
            }),
            TransferEvent::UpdateTransferSessions {
                loaded,
                added: new,
                removed,
                updated
            } => {
                let mut command = Command::new(|_| async move {});

                for loaded in loaded {
                    if model
                        .transfer
                        .transfer_sessions
                        .iter()
                        .any(|it| it.order_id == loaded.order_id && it.transfer_type.eq(&loaded.transfer_type))
                    {
                        continue;
                    }

                    model.transfer.transfer_sessions.push(loaded);
                }

                for new in new {
                    if model
                        .transfer
                        .transfer_sessions
                        .iter()
                        .any(|it| it.order_id == new.order_id && it.transfer_type.eq(&new.transfer_type))
                    {
                        log::info!("Already exists transfer session {:?}", new.order_id);
                        continue;
                    }

                    if new.transfer_type == TransferType::Receive {
                        let new = new.clone();
                        command = command.and(Command::new(|it| async move {
                            TransferSessionPersistentOperation::save(new).into_future(it.clone()).await;
                        }));
                    }

                    model.transfer.transfer_sessions.push(new);
                }

                model
                    .transfer
                    .transfer_sessions
                    .retain(|it| !removed.contains(&(it.order_id, it.transfer_type.clone())));

                for (order_id, transfer_type) in removed {
                    command = command.and(Command::new(|it| async move {
                        TransferSessionPersistentOperation::remove(order_id, transfer_type).into_future(it.clone()).await;
                    }));
                }

                for updated in updated {
                    let Some(session_pos) = model
                        .transfer
                        .transfer_sessions
                        .iter_mut()
                        .position(|it| it.order_id == updated.order_id && it.transfer_type.eq(&updated.transfer_type))
                    else {
                        continue;
                    };

                    log::info!("Update transfer session {:?}", updated.order_id);
                    model.transfer.transfer_sessions[session_pos] = updated;
                }

                model.transfer.transfer_sessions.sort_by(|a, b| b.order_id.cmp(&a.order_id));
                command.then_render()
            }
            TransferEvent::UpdateTransferTargets { added: new, removed } => {
                model.transfer.transfer_targets.extend(new);
                model.transfer.transfer_targets.retain(|it| !removed.iter().any(|removed| removed.id() == it.id()));
                Command::done()
            }
            TransferEvent::OpenSessionResource { session_id, resource_id } => {
                let Some(session) = model.transfer.transfer_sessions.iter().find(|it| it.order_id == session_id) else {
                    return Command::done();
                };

                let Some(resource) = session.resources.iter().find(|it| it.order_id == resource_id) else {
                    return Command::done();
                };

                let Some(transfer_progress) = session.progress.iter().find(|it| it.resource_order_id == resource_id) else {
                    return Command::done();
                };

                if !matches!(transfer_progress.status, TransferStatus::Success) {
                    return Command::done();
                }

                let resource_path = resource.path.clone();
                Command::new(move |it| async move {
                    let _ = OpenOperation::open(resource_path).into_future(it.clone()).await;
                })
            }
            TransferEvent::OpenSession { session_id } => {
                let Some(session) = model.transfer.transfer_sessions.iter().find(|it| it.order_id == session_id) else {
                    return Command::done();
                };

                if session.transfer_type == TransferType::Send {
                    return Command::done();
                }

                if !session.is_completed() {
                    return Command::new(|it| async move {
                        DialogOperation::toast("Session is not completed".to_string()).into_future(it.clone()).await;
                    });
                }

                let session_id = session.order_id;
                Command::new(|it| async move {
                    let _ = OpenOperation::open_session(session_id).into_future(it.clone()).await;
                })
            }
            TransferEvent::OpenSelectedResource { resource_id } => {
                let Some(resource) = model.transfer.selected_resources.iter().find(|it| it.order_id == resource_id) else {
                    return Command::done();
                };

                let resource_path = resource.path.clone();
                Command::new(move |it| async move {
                    let _ = OpenOperation::open(resource_path).into_future(it.clone()).await;
                })
            }
            TransferEvent::SessionResourceThumbnailFullFilled {
                session_id,
                resource_id,
                path
            } => {
                let Some(session) = model.transfer.transfer_sessions.iter_mut().find(|it| it.order_id == session_id) else {
                    return Command::done();
                };

                let resource = session.resources.iter_mut().find(|it| it.order_id == resource_id);
                if let Some(resource) = resource {
                    resource.thumbnail_path = Some(path.clone());
                    let resource = resource.clone();
                    return Command::new(|it| async move {
                        TransferSessionPersistentOperation::update_resource(session_id, resource)
                            .into_future(it.clone())
                            .await;
                    })
                    .then_render();
                }

                Command::done()
            }
            TransferEvent::FindPublicSession { keywords } => {
                let transfer_service = self.transfer_service;
                Command::new(|it| async move {
                    transfer_service.find_transfer_session(keywords, it.clone()).await;
                })
            }
            TransferEvent::ViewPublicSession {
                password,
                session_id,
                transfer_type
            } => {
                let Some(session) = model
                    .transfer
                    .transfer_sessions
                    .iter()
                    .find(|it| it.order_id == session_id && it.transfer_type.eq(&transfer_type))
                    .cloned()
                else {
                    return Command::done()
                };

                let transfer_service = self.transfer_service;
                Command::new(|it| async move {
                    transfer_service.view_public_session(session, password, it).await;
                })
            }
            TransferEvent::UpdateResourceTransferProgresses { session_id, progresses } => {
                let Some(session) = model.transfer.transfer_sessions.iter_mut().find(|it| it.order_id == session_id) else {
                    return Command::done();
                };

                for progress in progresses {
                    if let Some(index) = session.progress.iter().position(|it| it.resource_order_id == progress.resource_order_id) {
                        session.progress[index] = progress;
                    }
                }

                if session.is_completed() {
                    let progresses = session.progress.clone();
                    return Command::new(|it| async move {
                        TransferSessionPersistentOperation::update_progresses(session_id, progresses)
                            .into_future(it.clone())
                            .await;
                        it.notify_shell(CoreOperation::Render);
                    });
                }

                Command::new(|it| async move {
                    it.notify_shell(CoreOperation::Render);
                })
            }
        }
    }

    fn view(&self, model: &AppModel) -> Self::ViewModel {
        Self::ViewModel {
            is_loading_selected_resources: model.transfer.is_loading_selected_resources,
            selected_resources: model.transfer.selected_resources.iter().map(SelectedResourceViewModel::from).collect(),
            transfer_method_selection: model.transfer.transfer_method_selection.clone(),
            received_cloud_sessions: model
                .transfer
                .transfer_sessions
                .iter()
                .filter(|it| it.transfer_type == TransferType::Receive)
                .filter_map(|it| {
                    let (password, avatar, name, access_url, is_required_password, alias) = match &it.target {
                        TransferTarget::Internet {
                            password,
                            from_user,
                            access_url,
                            is_required_password
                        } => {
                            let Some(access_url) = access_url else {
                                return None;
                            };

                            let alias = match Url::parse(access_url) {
                                Ok(url) => {
                                    let alias = url.query_pairs().find(|it| it.0 == "session").map(|it| it.1.to_string());
                                    alias
                                }
                                Err(e) => None
                            };

                            let name = match &alias {
                                Some(alias) => format!("{} ({})", from_user.name, alias),
                                None => from_user.name.to_string()
                            };

                            (
                                password.clone(),
                                from_user.avatar.clone(),
                                name,
                                access_url,
                                *is_required_password,
                                alias
                            )
                        }
                        _ => return None
                    };

                    let image_resources = it
                        .resources
                        .iter()
                        .filter_map(|resource| {
                            if resource.r#type != ResourceType::Image {
                                return None;
                            }

                            let Some(progress) = it.progress.iter().find(|it| it.resource_order_id == resource.order_id) else {
                                return None;
                            };

                            Some(ImageReceiveResourceViewModel {
                                model: SelectedResourceViewModel::from(resource),
                                completion: progress.percentage() as f32,
                                is_completed: progress.status.is_completed()
                            })
                        })
                        .collect();

                    let video_resources = it
                        .resources
                        .iter()
                        .filter_map(|resource| {
                            if resource.r#type != ResourceType::Video {
                                return None;
                            }

                            let Some(progress) = it.progress.iter().find(|it| it.resource_order_id == resource.order_id) else {
                                return None;
                            };

                            Some(VideoReceiveResourceViewModel {
                                model: SelectedResourceViewModel::from(resource),
                                completion: progress.percentage() as f32,
                                is_completed: progress.status.is_completed()
                            })
                        })
                        .collect();

                    let file_resources = it
                        .resources
                        .iter()
                        .filter_map(|resource| {
                            if resource.r#type != ResourceType::File && resource.r#type != ResourceType::Folder {
                                return None;
                            }

                            let Some(progress) = it.progress.iter().find(|it| it.resource_order_id == resource.order_id) else {
                                return None;
                            };

                            Some(FileReceiveResourceViewModel {
                                model: SelectedResourceViewModel::from(resource),
                                completion: progress.percentage() as f32,
                                is_completed: progress.status.is_completed()
                            })
                        })
                        .collect();

                    Some(ReceiveCloudSessionViewModel {
                        id: it.order_id,
                        password,
                        avatar_url: avatar,
                        sender_name: name,
                        access_url: access_url.to_owned(),
                        is_required_password,
                        image_resources,
                        alias,
                        video_resources,
                        file_resources,
                        display_datetime: id_to_datetime(it.order_id)
                            .with_timezone(&chrono::Local)
                            .format("%Y-%m-%d %H:%M")
                            .to_string()
                    })
                })
                .collect::<Vec<_>>(),
            received_sessions: model
                .transfer
                .transfer_sessions
                .iter()
                .filter(|it| it.transfer_type == TransferType::Receive)
                .filter_map(|it| {
                    let Some(peer) = it.peer() else {
                        return None;
                    };

                    let image_resources = it
                        .resources
                        .iter()
                        .filter_map(|resource| {
                            if resource.r#type != ResourceType::Image {
                                return None;
                            }

                            let Some(progress) = it.progress.iter().find(|it| it.resource_order_id == resource.order_id) else {
                                return None;
                            };

                            Some(ImageReceiveResourceViewModel {
                                model: SelectedResourceViewModel::from(resource),
                                completion: progress.percentage() as f32,
                                is_completed: progress.status.is_completed()
                            })
                        })
                        .collect();

                    let video_resources = it
                        .resources
                        .iter()
                        .filter_map(|resource| {
                            if resource.r#type != ResourceType::Video {
                                return None;
                            }

                            let Some(progress) = it.progress.iter().find(|it| it.resource_order_id == resource.order_id) else {
                                return None;
                            };

                            Some(VideoReceiveResourceViewModel {
                                model: SelectedResourceViewModel::from(resource),
                                completion: progress.percentage() as f32,
                                is_completed: progress.status.is_completed()
                            })
                        })
                        .collect();

                    let file_resources = it
                        .resources
                        .iter()
                        .filter_map(|resource| {
                            if resource.r#type != ResourceType::File && resource.r#type != ResourceType::Folder {
                                return None;
                            }

                            let Some(progress) = it.progress.iter().find(|it| it.resource_order_id == resource.order_id) else {
                                return None;
                            };

                            Some(FileReceiveResourceViewModel {
                                model: SelectedResourceViewModel::from(resource),
                                completion: progress.percentage() as f32,
                                is_completed: progress.status.is_completed()
                            })
                        })
                        .collect();

                    Some(ReceiveSessionViewModel {
                        id: it.order_id,
                        peer_avatar: AvatarViewModel::new(peer.avatar_url.clone()),
                        peer_name: peer.name.clone().unwrap_or(peer.device.name.clone()),
                        peer_description: "Nearby".to_owned(),
                        is_completed: it.is_completed(),
                        is_in_progress: !it.is_completed() && !it.is_canceled(),
                        display_download_speed: it.status().to_string(),
                        progress: it.total_progress(),
                        image_resources,
                        video_resources,
                        file_resources,
                        display_datetime: id_to_datetime(it.order_id)
                            .with_timezone(&chrono::Local)
                            .format("%Y-%m-%d %H:%M")
                            .to_string()
                    })
                })
                .collect(),
            cloud_session: model
                .transfer
                .transfer_sessions
                .iter()
                .filter(|it| matches!(it.transfer_type, TransferType::Send))
                .filter(|it| it.target.is_public())
                .find_map(|it| {
                    let (access_url, password) = match &it.target {
                        TransferTarget::Internet { access_url, password, .. } => (access_url.clone(), password.clone()),
                        _ => return None
                    };

                    Some(CloudSession {
                        display_download_speed: match access_url.is_none() {
                            true => "Initializing...".to_owned(),
                            false => it.status().to_string()
                        },
                        password,
                        session_id: it.order_id,
                        is_completed: it.is_completed(),
                        is_in_progress: !it.is_completed() && !it.is_canceled(),
                        progress: it.total_progress(),
                        access_url
                    })
                }),
            nearby_peers: model
                .transfer
                .transfer_targets
                .iter()
                .filter_map(|it| match it {
                    TransferTarget::Nearby(peer) => {
                        let send_session = model
                            .transfer
                            .transfer_sessions
                            .iter()
                            .filter(|it| it.target.is_peer())
                            .find(|it| it.transfer_type == TransferType::Send && *it.peer_id().as_ref().unwrap() == peer.id);

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
