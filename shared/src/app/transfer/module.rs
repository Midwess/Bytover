use std::collections::HashMap;
use crate::app::core::extensions::{CoreCommandContextUtils, CoreCommandUtils};
use crate::app::core::model_events::{TransferSessionModelEvent, TransferSessionUpdateEvent, UpdateAction};
use crate::app::modules::AppModule;
use crate::app::nearby::module::NearbyEvent;
use crate::app::operations::device::DeviceOperation;
use crate::app::operations::dialog::{AlertDialog, DialogOperation};
use crate::app::operations::rpc::RpcOperation;
use crate::app::view_models::cloud_session::CloudSession;
use crate::app::view_models::receive_session::{
    FileReceiveResourceViewModel,
    ImageReceiveResourceViewModel,
    ReceiveSessionViewModel,
    VideoReceiveResourceViewModel
};
use crate::app::view_models::selected_resource::SelectedResourceViewModel;
use crate::app::{AppEvent, AppModel, BitBridge};
use crate::entities::finding_scope::FindingScope;
use crate::entities::local_resource::{LocalResource, ResourceType};
use crate::entities::target::{P2PConnectionState, TransferTarget};
use crate::entities::transfer_method::TransferMethodSelection;
use crate::entities::transfer_session::{TransferSession, TransferSessionStatus, TransferStatus, TransferType};
use crate::repository::transfer_session::TransferSessionId;
use core_services::db::repository::abstraction::id::{DbId, VecTableLookup};
use core_services::db::repository::abstraction::table::Table;
use crux_core::{App, Command};
use devlog_sdk::distributed_id::id_to_datetime;
use serde::{Deserialize, Serialize};
use crate::app::operations::persistent::TransferSessionPersistentOperation;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TransferModel {
    selected_method: TransferMethodSelection,
    pub sessions: Vec<TransferSession>,
    keywords: String,
    pub selected_receive_session_id: Option<u64>
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct TransferViewModel {
    transfer_method: TransferMethodSelection,
    received_sessions: Vec<ReceiveSessionViewModel>,
    received_cloud_sessions: Vec<ReceiveSessionViewModel>,
    cloud_session: Option<CloudSession>,
    p2p_sessions: Vec<CloudSession>,
    selected_session: Option<ReceiveSessionViewModel>
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
    StartP2PTransfer {
        nearby_available: bool,
        password: Option<String>
    },
    CancelTransfer {
        session_id: u64,
        transfer_type: TransferType
    },
    TransferCanceled {
        session_id: u64
    },
    OpenResource {
        session_id: u64,
        resource_id: u64
    },
    FindPublicSession {
        keywords: String
    },
    ViewSession {
        password: Option<String>,
        session_id: u64,
        transfer_type: TransferType
    },
    Clear,
    ReceivedViewSessionRequest {
        peer_id: String,
        request_id: String,
        order_id: u64,
        password: Option<String>
    },
    RequestSessionDetail {
        peer_id: String,
        order_id: u64,
        password: Option<String>
    },
    ReceivedDownloadRequest {
        peer_id: String,
        session_order_id: u64,
        resource_order_id: u64,
        transfer_id: u16
    },
    ResourceNotification {
        session_order_id: u64,
        resource: LocalResource,
        peer_id: String,
    },
    RequestDownloadResource {
        peer_id: String,
        session_order_id: u64,
        resource_order_id: u64
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
            TransferEvent::Launch => Command::handle_result(|it| async move { it.app().load_transfer_sessions().await }),
            TransferEvent::Clear => {
                model.transfer.sessions.clear();
                Command::handle_result(|it| async move {
                    let _  = it.app().run(TransferSessionPersistentOperation::clear_all()).await;
                    Ok(())
                })
            },
            TransferEvent::CancelTransfer { session_id, transfer_type} => {
                let id = TransferSessionId {
                    order_id: Some(session_id.to_string()),
                    transfer_type: Some(transfer_type),
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

                Command::handle_result(|it| async move {
                    if !session.is_completed() {
                        let confirmation = DialogOperation::alert(AlertDialog::confirmation(
                            "Cancel the transfer ?".to_string(),
                            "Yes".to_string(),
                            Some("No".to_string())
                        ))
                        .into_future(it.clone())
                        .await;

                        if !confirmation {
                            log::info!("User not agree to cancel transfer");
                            return Ok(());
                        }
                    }

                    it.app().delete_session(&session).await
                })
            }
            TransferEvent::DeleteSession { session_id } => {
                let id = TransferSessionId {
                    order_id: Some(session_id.to_string()),
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

                Command::handle_result(|it| async move { it.app().delete_session(&session).await })
            }
            TransferEvent::TransferCanceled { session_id, .. } => {
                let id = TransferSessionId {
                    order_id: Some(session_id.to_string()),
                    ..Default::default()
                };
                let Some(session) = model.transfer.sessions.lookup_mut(&id) else {
                    return Command::done();
                };

                session.cancel();

                let session = session.clone();
                Command::handle_result(|it| async move { it.app().delete_session(&session).await })
            }
            TransferEvent::StartPublicTransfer { password, to_emails } => {
                let selected_resources = model.shelf.shelf.resources.clone();
                let Some(user) = model.authentication.user.clone() else {
                    log::info!("User is not login, open login page");
                    return Command::handle_result(|it| async move {
                        it.app().authenticate().await;
                        Ok(())
                    });
                };

                Command::handle_result(|it| async move {
                    let session = TransferSession::public(user, password, selected_resources, to_emails);
                    it.app().upload(session).await
                })
            }
            TransferEvent::StartP2PTransfer { password, .. } => {
                let selected_resources = model.shelf.shelf.resources.clone();
                if selected_resources.is_empty() {
                    return Command::new(|it| async move {
                        let _ = DialogOperation::toast("No resources selected".to_string()).into_future(it.clone()).await;
                    });
                }

                // Check if user is authenticated - if not, trigger sign-in flow
                let user = model.authentication.user.clone();
                if user.is_none() {
                    log::info!("User is not logged in, opening login page");
                    return Command::handle_result(|it| async move {
                        it.app().authenticate().await;
                        Ok(())
                    });
                }

                let Some(_me) = model.nearby.me.clone() else {
                    log::info!("Nearby service not available");
                    return Command::done()
                };

                Command::handle_result(move |it| async move {
                    let p2p_session = it.app().run(RpcOperation::create_p2p_session()).await?;

                    let mut session = TransferSession::p2p(
                        selected_resources,
                        password,
                        p2p_session.signalling_room_id.clone(),
                        p2p_session.signalling_scope.clone(),
                        p2p_session.alias.clone(),
                        String::new(),
                        p2p_session.session_id,
                    );

                    let scope = FindingScope::new(&p2p_session.signalling_room_id);
                    it.notify_event(NearbyEvent::AddFindingScope(scope));

                    session.from_user = user.unwrap();
                    it.update_model(TransferSessionModelEvent::Add(session.clone()));

                    Ok(())
                })
            }
            TransferEvent::ModelEvent(event) => {
                match event {
                    TransferSessionModelEvent::Update(session_id, action) => {
                        let should_persist = matches!(action, crate::app::core::model_events::TransferSessionUpdateEvent::SessionDetailUpdated(_));

                        if let Some(session) = model.transfer.sessions.lookup_mut(&session_id) {
                            action.update(session);

                            if should_persist {
                                let session_clone = session.clone();
                                model.transfer.sessions.sort_by(|a, b| b.order_id.cmp(&a.order_id));
                                return Command::handle_result(|it| async move {
                                    let _ = it.app().run(TransferSessionPersistentOperation::save(session_clone)).await;
                                    Ok(())
                                }).then(Command::render());
                            }
                        }
                    }
                    TransferSessionModelEvent::Add(new) => {
                        if model.transfer.sessions.iter().any(|it| it.id().is_represent(&new)) {
                            return Command::done();
                        }

                        model.transfer.sessions.push(new);
                    }
                    TransferSessionModelEvent::Remove(session_id) => {
                        model.transfer.sessions.retain(|it| !session_id.is_represent(it));
                    }
                }

                model.transfer.sessions.sort_by(|a, b| b.order_id.cmp(&a.order_id));

                Command::render()
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
                    let _ = DeviceOperation::open(resource_path).into_future(it.clone()).await;
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
                    let _ = DeviceOperation::open_session(session_id).into_future(it.clone()).await;
                })
            }
            TransferEvent::FindPublicSession { mut keywords } => {
                if let Ok(url) = url::Url::parse(&keywords) {
                    let Some(query) = url.query_pairs().find(|(key, _)| key == "session").map(|it| it.1.to_string()) else {
                        log::info!("Not found query key session");
                        return Command::done()
                    };

                    keywords = query;
                }

                model.transfer.keywords = keywords.clone();
                log::info!("Find public session with keywords: {}", keywords);
                if model.transfer.sessions.iter().any(|it| matches!(it.transfer_type, TransferType::Receive) && it.is_keyword_match(&keywords)) {
                    return Command::render();
                }

                Command::handle_result(|it| async move { it.app().find_transfer_session(keywords).await }).then_render()
            }
            TransferEvent::ViewSession { password, session_id, .. } => {
                log::info!("View session: {}", session_id);
                let session_id = TransferSessionId {
                    order_id: Some(session_id.to_string()),
                    transfer_type: Some(TransferType::Receive)
                };

                let Some(mut session) = model.transfer.sessions.lookup(&session_id).cloned() else {
                    log::info!("Session {:?} not found", session_id);
                    return Command::done()
                };

                model.transfer.selected_receive_session_id = Some(session.order_id);

                if let TransferTarget::P2P { scope, .. } = &mut session.target {
                    scope.set_watcher(false);
                };

                match &session.target {
                    TransferTarget::P2P { connection_state, scope, from_peer, .. } => {
                        let should_request = match connection_state {
                            P2PConnectionState::NotConnected |
                            P2PConnectionState::Failed(_) => true,
                            P2PConnectionState::Connected => session.resources.is_empty(),
                            P2PConnectionState::Connecting => false
                        };

                        if !should_request {
                            return Command::done();
                        }

                        if from_peer.is_none() {
                            return Command::event(AppEvent::Nearby(NearbyEvent::AddFindingScope(scope.clone())));
                        }

                        let peer_id = from_peer.as_ref().unwrap().id().to_string();
                        let session_id_clone = session_id.clone();

                        Command::handle_result(move |it| async move {
                            it.app().request_session_detail(peer_id, session_id_clone, session.order_id, password).await
                        })
                    }
                    TransferTarget::Internet { .. } => {
                        Command::handle_result(|it| async move {
                            it.app().view_public_session(session, password).await
                        })
                    }
                }
            }
            TransferEvent::ReceivedViewSessionRequest { peer_id, request_id, order_id, password } => {
                let session_id = TransferSessionId {
                    order_id: Some(order_id.to_string()),
                    transfer_type: Some(TransferType::Send)
                };

                let session = model.transfer.sessions.lookup(&session_id).cloned();
                let device = model.environment.device.clone();
                Command::handle_result(move |it| async move {
                    it.app().handle_view_session_request(peer_id, request_id, password, session, device).await
                })
            }
            TransferEvent::RequestSessionDetail { peer_id, order_id, password } => {
                let session_id = TransferSessionId {
                    order_id: Some(order_id.to_string()),
                    transfer_type: Some(TransferType::Receive)
                };

                if let Some(session) = model.transfer.sessions.lookup(&session_id) {
                    if let TransferTarget::P2P { ref scope, .. } = session.target {
                        let mut scope = scope.clone();
                        scope.set_watcher(false);
                        model.nearby.finding_scopes.retain(|s| s.scope_id() != scope.scope_id());
                        model.nearby.finding_scopes.push(scope.clone());
                        return Command::event(AppEvent::Nearby(NearbyEvent::AddFindingScope(scope))).then(
                            Command::handle_result(move |it| async move {
                                it.app().request_session_detail(peer_id, session_id, order_id, password).await
                            })
                        );
                    }
                }

                Command::handle_result(move |it| async move {
                    it.app().request_session_detail(peer_id, session_id, order_id, password).await
                })
            }
            TransferEvent::ReceivedDownloadRequest { peer_id, session_order_id, resource_order_id, transfer_id } => {
                let session_id = TransferSessionId {
                    order_id: Some(session_order_id.to_string()),
                    transfer_type: Some(TransferType::Send)
                };

                let resource = model
                    .transfer
                    .sessions
                    .lookup(&session_id)
                    .and_then(|s| s.resources.iter().find(|r| r.order_id == resource_order_id).cloned());

                Command::handle_result(move |it| async move {
                    it.app().handle_download_request(peer_id, session_order_id, transfer_id, resource).await
                })
            }
            TransferEvent::ResourceNotification { session_order_id, resource, peer_id } => {
                let session_id = TransferSessionId {
                    order_id: Some(session_order_id.to_string()),
                    transfer_type: Some(TransferType::Receive)
                };

                let Some(session) = model.transfer.sessions.lookup(&session_id) else {
                    log::warn!("Session {} not found for resource notification", session_order_id);
                    return Command::done();
                };

                let Some(peer) = model.nearby.peers.iter().find(|p| p.id == peer_id).cloned() else {
                    log::warn!("Peer {} not found, ignoring resource notification", peer_id);
                    return Command::done();
                };

                if !peer.is_owned(session) {
                    log::warn!("Peer {} is not owner of session {}, ignoring resource notification", peer_id, session_order_id);
                    return Command::done();
                }

                let resource_order_id = resource.order_id;
                let resource_name = resource.name.clone();

                Command::handle_result(move |it| async move {
                    let mut generate_file_paths_request = HashMap::new();
                    generate_file_paths_request.insert(resource_order_id, resource_name);

                    let mut generated_saved_paths = it.app().run(
                        TransferSessionPersistentOperation::generate_resource_paths(
                            session_order_id,
                            generate_file_paths_request
                        )
                    ).await?;

                    let Some(generated_path) = generated_saved_paths.remove(&resource_order_id) else {
                        log::warn!("Failed to generate path for resource {}", resource_order_id);
                        return Ok(());
                    };

                    let mut updated_resource = resource;
                    updated_resource.path = generated_path;

                    it.update_model(TransferSessionModelEvent::Update(
                        session_id,
                        TransferSessionUpdateEvent::ResourceUpdate(updated_resource)
                    ));

                    Ok(())
                })
            }
            TransferEvent::RequestDownloadResource { peer_id, session_order_id, resource_order_id } => {
                let id = TransferSessionId {
                    order_id: Some(session_order_id.to_string()),
                    transfer_type: Some(TransferType::Receive)
                };

                let Some(resource) = model.transfer.sessions.lookup(&id).and_then(|s| s.resources.iter().find(|r| r.order_id == resource_order_id).cloned()) else {
                    log::warn!("Resource not found in session: {}", resource_order_id);
                    return Command::done();
                };

                Command::handle_result(move |it| async move {
                    it.app().request_download_resource(peer_id, id, resource).await
                })
            }
        }
    }

    fn view(&self, model: &AppModel) -> Self::ViewModel {
        let received_sessions = model
            .transfer
            .sessions
            .iter()
            .filter(|it| it.transfer_type == TransferType::Receive)
            .filter_map(|it| {
                let from_user = &it.from_user;
                let is_p2p = it.is_p2p_connected();

                let status = it.status();
                let is_loading = matches!(status, TransferSessionStatus::Initializing { .. });
                let loading_status = if let TransferSessionStatus::Initializing { loading_state: Some(text), .. } = &status {
                    Some(text.clone())
                } else {
                    None
                };

                let error_message = if let TransferSessionStatus::Failed(msg) = &status {
                    Some(msg.clone())
                } else if let TransferSessionStatus::Initializing { loading_error: Some(error), .. } = &status {
                    Some(error.clone())
                } else {
                    None
                };

                let (sender_id, sender_avatar, sender_name, sender_description, alias, access_url, password, is_required_password) = match &it.target {
                    TransferTarget::P2P { from_peer, .. } => {
                        let sender_id = from_peer.as_ref().map(|p| p.id().to_string()).unwrap_or_else(|| from_user.id.to_string());
                        let alias = if !it.alias.is_empty() { Some(it.alias.clone()) } else { None };
                        (sender_id, from_user.avatar.clone(), from_user.name.clone(), it.description.clone().unwrap_or_default(), alias, None, None, it.is_required_password)
                    }
                    TransferTarget::Internet { .. } => {
                        let access_url_ref = if !it.access_url.is_empty() { Some(it.access_url.clone()) } else { None };
                        let alias = if !it.alias.is_empty() { Some(it.alias.clone()) } else { None };
                        let name = match &alias {
                            Some(a) => format!("{} ({})", from_user.name, a),
                            None => from_user.name.to_string()
                        };
                        (from_user.id.to_string(), from_user.avatar.clone(), name, "Public".to_string(), alias, access_url_ref, it.password.clone(), it.is_required_password)
                    }
                };

                let image_resources = it.resources.iter().filter_map(|resource| {
                    let progress = it.progress.iter().find(|p| p.resource_order_id == resource.order_id)?;

                    if resource.r#type != ResourceType::Image { return None; }
                    Some(ImageReceiveResourceViewModel {
                        model: SelectedResourceViewModel::from(resource),
                        completion: progress.percentage() as f32,
                        is_ready: is_p2p || progress.status.is_completed(),
                        is_completed: progress.status.is_completed(),
                    })
                }).collect();

                let video_resources = it.resources.iter().filter_map(|resource| {
                    if resource.r#type != ResourceType::Video { return None; }
                    let progress = it.progress.iter().find(|p| p.resource_order_id == resource.order_id)?;
                    Some(VideoReceiveResourceViewModel {
                        model: SelectedResourceViewModel::from(resource),
                        completion: progress.percentage() as f32,
                        is_completed: progress.status.is_completed(),
                        is_ready: is_p2p || progress.status.is_completed(),
                    })
                }).collect();

                let file_resources = it.resources.iter().filter_map(|resource| {
                    if !matches!(resource.r#type, ResourceType::File | ResourceType::Folder) { return None; }
                    let progress = it.progress.iter().find(|p| p.resource_order_id == resource.order_id)?;

                    Some(FileReceiveResourceViewModel {
                        model: SelectedResourceViewModel::from(resource),
                        completion: progress.percentage() as f32,
                        is_completed: progress.status.is_completed(),
                        is_ready: is_p2p || progress.status.is_completed(),
                    })
                }).collect();

                Some(ReceiveSessionViewModel {
                    is_cloud: it.target.is_public(),
                    is_scope_online: match &it.target {
                        TransferTarget::P2P { scope, .. } => scope.is_online(),
                        _ => false
                    },
                    id: it.order_id.to_string(),
                    sender_id,
                    sender_avatar,
                    sender_name,
                    sender_description,
                    alias,
                    access_url,
                    password,
                    password_required: is_required_password,
                    is_authenticated: !it.resources.is_empty(),
                    has_details: !it.resources.is_empty(),
                    is_loading,
                    loading_status,
                    error_message,
                    is_completed: it.is_completed(),
                    is_in_progress: matches!(status, TransferSessionStatus::InProgress { .. }),
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
            .collect::<Vec<_>>();

        let (received_cloud_sessions, mut received_sessions): (Vec<_>, Vec<_>) =
            received_sessions.into_iter().partition(|it| it.is_cloud);

        // Sort P2P sessions: online ones first
        received_sessions.sort_by(|a, b| {
            match (a.is_scope_online, b.is_scope_online) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => std::cmp::Ordering::Equal,
            }
        });

        let selected_session = model.transfer.selected_receive_session_id.and_then(|selected_id| {
            received_sessions.iter()
                .chain(received_cloud_sessions.iter())
                .find(|s| s.id == selected_id.to_string())
                .cloned()
        });

        Self::ViewModel {
            transfer_method: model.transfer.selected_method.clone(),
            received_sessions,
            received_cloud_sessions,
            selected_session,
            cloud_session: model.transfer.sessions.iter()
                .filter(|it| matches!(it.transfer_type, TransferType::Send))
                .filter(|it| it.target.is_public())
                .find_map(|it| {
                    match &it.target {
                        TransferTarget::Internet { .. } => {
                            let access_url = if !it.access_url.is_empty() { Some(it.access_url.clone()) } else { None };
                            let status = it.status();
                            Some(CloudSession {
                                display_download_speed: match access_url.is_none() {
                                    true => "Initializing...".to_owned(),
                                    false => status.to_string()
                                },
                                password: it.password.clone(),
                                session_id: it.order_id.to_string(),
                                is_completed: it.is_completed(),
                                is_in_progress: matches!(status, TransferSessionStatus::InProgress { .. }),
                                progress: it.total_progress(),
                                access_url
                            })
                        },
                        _ => None
                    }
                }),
            p2p_sessions: model.transfer.sessions.iter()
                .filter(|it| matches!(it.transfer_type, TransferType::Send))
                .filter(|it| it.target.is_peer())
                .filter_map(|it| {
                    let status = it.status();
                    Some(CloudSession {
                        display_download_speed: status.to_string(),
                        password: it.password.clone(),
                        session_id: it.order_id.to_string(),
                        is_completed: it.is_completed(),
                        is_in_progress: !it.is_completed(),
                        progress: it.total_progress(),
                        access_url: None
                    })
                })
                .collect(),
        }
    }
}
