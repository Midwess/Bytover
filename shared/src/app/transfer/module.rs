use crate::app::core::extensions::{CoreCommandContextUtils, CoreCommandUtils};
use crate::app::core::model_events::{TransferSessionModelEvent, UpdateAction};
use crate::app::modules::AppModule;
use crate::app::operations::device::OpenOperation;
use crate::app::operations::dialog::{AlertDialog, DialogOperation};
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
use crate::app::{AppModel, BitBridge};
use crate::entities::local_resource::ResourceType;
use crate::entities::peer::Peer;
use crate::entities::target::TransferTarget;
use crate::entities::transfer_method::TransferMethodSelection;
use crate::entities::transfer_session::{TransferSession, TransferStatus, TransferType};
use crate::repository::transfer_session::{TransferSessionId, TransferTargetId};
use core_services::db::repository::abstraction::id::{DbId, VecTableLookup};
use crux_core::{App, Command};
use devlog_sdk::distributed_id::id_to_datetime;
use schema::devlog::bitbridge::TransferSessionMessage;
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TransferModel {
    selected_method: TransferMethodSelection,
    sessions: Vec<TransferSession>,
    targets: Vec<TransferTarget>
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct TransferViewModel {
    transfer_method: TransferMethodSelection,
    nearby_peers: Vec<PeerViewModel>,
    received_sessions: Vec<ReceiveSessionViewModel>,
    received_cloud_sessions: Vec<ReceiveCloudSessionViewModel>,
    cloud_session: Option<CloudSession>
}

pub struct TransferModule;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum TransferEvent {
    Launch,
    OpenSession {
        session_id: u64
    },
    DeleteSession {
        session_id: u64
    },
    StartPublicTransfer {
        password: Option<String>,
        to_emails: Vec<String>
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
    UpdateTransferTargets {
        added: Vec<TransferTarget>,
        removed: Vec<TransferTarget>
    },
    OpenResource {
        session_id: u64,
        resource_id: u64
    },
    FindPublicSession {
        keywords: String
    },
    ViewPublicSession {
        password: Option<String>,
        session_id: u64,
        transfer_type: TransferType
    },

    #[serde(skip)]
    ModelEvent(TransferSessionModelEvent)
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
            TransferEvent::Launch => Command::new(|it| async move {
                it.app().load_transfer_sessions().await;
            }),
            TransferEvent::CancelTransfer { session_id, transfer_type } => {
                let id = TransferSessionId {
                    order_id: Some(session_id),
                    r#type: Some(transfer_type),
                    ..Default::default()
                };
                let Some(session) = model.transfer.sessions.lookup(&id).cloned() else {
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

                    it.app().delete_session(session).await;
                })
            }
            TransferEvent::DeleteSession { session_id } => {
                let id = TransferSessionId {
                    order_id: Some(session_id),
                    ..Default::default()
                };
                let Some(session) = model.transfer.sessions.lookup(&id).cloned() else {
                    return Command::done();
                };

                if !session.is_completed() {
                    return Command::new(|it| async move {
                        DialogOperation::toast("Session is still in progress".to_string()).into_future(it.clone()).await;
                    });
                }

                Command::new(|it| async move {
                    it.app().delete_session(session).await;
                })
            }
            TransferEvent::TransferCanceled { session_id, .. } => {
                let id = TransferSessionId {
                    order_id: Some(session_id),
                    ..Default::default()
                };
                let Some(session) = model.transfer.sessions.lookup_mut(&id) else {
                    return Command::done();
                };

                session.cancel();

                let session = session.clone();
                Command::new(|it| async move {
                    it.app().delete_session(session).await;
                })
            }
            TransferEvent::StartPublicTransfer { password, to_emails } => {
                let selected_resources = model.shelf.shelf.resources.clone();
                let Some(user) = model.authentication.user.clone() else {
                    return Command::operate(DialogOperation::Toast("Unauthenticated".to_owned()))
                };

                let target = TransferTarget::Internet {
                    is_required_password: password.is_some(),
                    password,
                    access_url: None,
                    from_user: user.clone(),
                    to_emails
                };

                Command::new(|it| async move {
                    it.app().transfer(user, selected_resources, target).await;
                })
            }
            TransferEvent::StartTransfer { target_id } => {
                let selected_resources = model.shelf.shelf.resources.clone();
                let transfer_targets = model.transfer.targets.clone();
                let Some(target) = transfer_targets.iter().find(|it| it.id() == target_id).cloned() else {
                    return Command::done();
                };

                let duplicated_session = model
                    .transfer
                    .sessions
                    .iter()
                    .filter(|it| it.transfer_type == TransferType::Send)
                    .find(|it| it.peer_id().map(|id| id.to_string()) == Some(target.id()))
                    .cloned();

                let Some(user) = model.authentication.user.clone() else {
                    return Command::operate(DialogOperation::Toast("unauthenticated".to_owned()));
                };

                Command::new(|it| async move {
                    if let Some(duplicated_session) = duplicated_session {
                        it.notify_event(TransferEvent::CancelTransfer {
                            session_id: duplicated_session.order_id,
                            transfer_type: duplicated_session.transfer_type
                        });
                        return;
                    }

                    it.app().transfer(user, selected_resources, target).await;
                })
            }
            TransferEvent::TransferRequest { remote_session, peer } => Command::new(|it| async move {
                it.app().accept_session(remote_session, peer).await;
            }),
            TransferEvent::ModelEvent(event) => {
                match event {
                    TransferSessionModelEvent::Update(session_id, action) => {
                        if let Some(session) = model.transfer.sessions.lookup_mut(&session_id) {
                            action.update(session);
                        }
                    }
                    TransferSessionModelEvent::Add(new) => {
                        model.transfer.sessions.push(new);
                    }
                    TransferSessionModelEvent::Remove(session_id) => {
                        model.transfer.sessions.retain(|it| !session_id.is_represent(it));
                    }
                }

                Command::done()
            }
            TransferEvent::UpdateTransferTargets { added: new, removed } => {
                model.transfer.targets.extend(new);
                model.transfer.targets.retain(|it| !removed.iter().any(|removed| removed.id() == it.id()));
                Command::done()
            }
            TransferEvent::OpenResource { session_id, resource_id } => {
                let Some(session) = model.transfer.sessions.iter().find(|it| it.order_id == session_id) else {
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
                let Some(session) = model.transfer.sessions.iter().find(|it| it.order_id == session_id) else {
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
            TransferEvent::FindPublicSession { keywords } => Command::new(|it| async move {
                it.app().find_transfer_session(keywords).await;
            }),
            TransferEvent::ViewPublicSession { password, session_id, .. } => {
                let session_id = TransferSessionId {
                    target: Some(TransferTargetId::Internet),
                    order_id: Some(session_id),
                    r#type: Some(TransferType::Receive)
                };

                let Some(session) = model.transfer.sessions.lookup(&session_id).cloned() else {
                    return Command::done()
                };

                Command::new(|it| async move {
                    it.app().view_public_session(session, password).await;
                })
            }
        }
    }

    fn view(&self, model: &AppModel) -> Self::ViewModel {
        Self::ViewModel {
            transfer_method: model.transfer.selected_method.clone(),
            received_cloud_sessions: model
                .transfer
                .sessions
                .iter()
                .filter(|it| it.transfer_type == TransferType::Receive)
                .filter_map(|it| {
                    let (password, avatar, name, access_url, is_required_password, alias, _to_emails) = match &it.target {
                        TransferTarget::Internet {
                            password,
                            from_user,
                            access_url,
                            is_required_password,
                            to_emails
                        } => {
                            let Some(access_url) = access_url else {
                                return None;
                            };

                            let alias = match Url::parse(access_url) {
                                Ok(url) => {
                                    let alias = url.query_pairs().find(|it| it.0 == "session").map(|it| it.1.to_string());
                                    alias
                                }
                                Err(_e) => None
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
                                alias,
                                to_emails.clone()
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
                .sessions
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
                .sessions
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
                .targets
                .iter()
                .filter_map(|it| match it {
                    TransferTarget::Nearby(peer) => {
                        let send_session = model
                            .transfer
                            .sessions
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
