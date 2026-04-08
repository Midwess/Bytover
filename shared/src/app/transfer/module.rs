use crate::app::core::extensions::{CoreCommandContextUtils, CoreCommandUtils};
use crate::app::core::model_events::{ConnectionRecovered, TransferSessionModelEvent, TransferSessionUpdateEvent, UpdateAction};
use crate::app::modules::AppModule;
use crate::app::operations::device::DeviceOperation;
use crate::app::operations::dialog::{AlertDialog, DialogOperation};
use crate::app::operations::p2p::P2POperation;
use crate::app::operations::persistent::TransferSessionPersistentOperation;
use crate::app::operations::CoreOperation;
use crate::app::view_models::cloud_session::CloudSession;
use crate::app::view_models::receive_session::{ReceiveResourceViewModel, ReceiveSessionViewModel};
use crate::app::view_models::selected_resource::SelectedResourceViewModel;
use crate::app::{AppModel, BitBridge};
use crate::entities::local_resource::{LocalResource, LocalResourcePath, ResourceType};
use crate::entities::peer::Peer;
use crate::entities::target::{P2PConnectionState, TransferTarget};
use crate::entities::transfer_method::TransferMethodSelection;
use crate::entities::transfer_session::{TransferSession, TransferSessionStatus, TransferStatus, TransferType};
use crate::repository::transfer_session::TransferSessionId;
use core_services::db::repository::abstraction::id::{DbId, VecTableLookup};
use core_services::db::repository::abstraction::table::Table;
use crux_core::{App, Command};
use devlog_sdk::distributed_id::id_to_datetime;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TransferModel {
    selected_method: TransferMethodSelection,
    pub sessions: Vec<TransferSession>
}

impl TransferModel {
    pub fn has_active_send_session(&self, shelf_id: u64) -> bool {
        self.sessions.iter().any(|s| {
            if s.is_completed() {
                return false
            }

            if let TransferType::Send { from_shelf_id } = &s.transfer_type {
                return from_shelf_id == &shelf_id;
            }

            false
        })
    }

    pub fn get_active_p2p_send_session(&self, shelf_id: u64) -> Option<&TransferSession> {
        self.sessions.iter().find(|s| {
            matches!(s.transfer_type, TransferType::Send { from_shelf_id } if from_shelf_id == shelf_id) &&
                s.target.is_peer() &&
                !s.is_completed()
        })
    }

    pub fn count_active_p2p_sessions(&self) -> usize {
        self.sessions.iter().filter(|s| s.target.is_peer() && !s.is_completed()).count()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct TransferViewModel {
    transfer_method: TransferMethodSelection,
    pub received_session: Option<ReceiveSessionViewModel>,
    pub received_cloud_session: Option<ReceiveSessionViewModel>,
    cloud_sessions: Vec<CloudSession>,
    p2p_sessions: Vec<CloudSession>,
    pub total_p2p_receive_progress: Option<f64>,
    pub is_loading: bool
}

pub struct TransferModule;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum TransferEvent {
    Launch,
    StartPublicTransfer {
        shelf_id: u64,
        password: Option<String>,
        to_emails: Vec<String>
    },
    StartP2PTransfer {
        shelf_id: u64,
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
    FindSession {
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
        peer_id: String
    },
    RequestDownloadResource {
        peer_id: String,
        session_order_id: u64,
        resource_order_id: u64
    },
    CancelResourceTransfer {
        session_id: u64,
        transfer_type: TransferType,
        resource_id: Option<u64>
    },
    RequestDownloadAllResources {
        peer_id: String,
        session_order_id: u64
    },
    NewTransferResource {
        shelf_id: u64,
        resource: LocalResource
    },

    UpdateConnectionState {
        session_id: u64,
        state: P2PConnectionState
    },
    PeerConnected {
        session_id: u64,
        peer: Peer
    },
    PeerDisconnected {
        peer_id: String
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
                Command::render()
            }
            TransferEvent::CancelTransfer { session_id, transfer_type } => {
                let id = TransferSessionId {
                    order_id: Some(session_id.to_string()),
                    transfer_type: Some(transfer_type)
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

                    let _ = it.app().cancel_resource_transfer(&session, None).await;
                    it.app().cancel_session(&session).await
                })
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
                Command::handle_result(|it| async move { it.app().cancel_session(&session).await })
            }
            TransferEvent::StartPublicTransfer {
                shelf_id,
                password,
                to_emails
            } => {
                let Some(shelf) = model.shelf.get_shelf(shelf_id) else {
                    return Command::operate(DialogOperation::Toast("Shelf not found.".to_owned()));
                };
                let selected_resources = shelf.resources.clone();
                let Some(user) = model.authentication.user.clone() else {
                    log::info!("User is not login, open login page");
                    return Command::handle_result(|it| async move {
                        it.app().authenticate().await;
                        Ok(())
                    });
                };

                Command::handle_result(move |it| async move {
                    let session = TransferSession::public(user, password, selected_resources, to_emails, shelf_id);
                    it.app().upload(session).await
                })
            }
            TransferEvent::StartP2PTransfer {
                shelf_id,
                nearby_available: _,
                password
            } => {
                use crate::app::transfer::commands::MAX_CONCURRENT_P2P_SESSIONS;

                let Some(shelf) = model.shelf.get_shelf(shelf_id) else {
                    return Command::operate(DialogOperation::Toast("Shelf not found.".to_owned()));
                };

                log::info!("Start P2P transfer for {shelf_id} with alias {}", shelf.name);
                let selected_resources = shelf.resources.clone();
                let shelf_name = shelf.name.clone();
                if selected_resources.is_empty() {
                    return Command::new(|it| async move {
                        let _ = DialogOperation::toast("No resources selected".to_string()).into_future(it.clone()).await;
                    });
                }

                let active_p2p_count = model.transfer.count_active_p2p_sessions();
                if active_p2p_count >= MAX_CONCURRENT_P2P_SESSIONS {
                    return Command::new(|it| async move {
                        let _ = DialogOperation::toast(format!(
                            "Maximum {} P2P sessions reached. Please complete or cancel an existing session first.",
                            MAX_CONCURRENT_P2P_SESSIONS
                        ))
                        .into_future(it.clone())
                        .await;
                    });
                }

                let Some(user) = model.authentication.user.clone() else {
                    log::info!("User is not logged in, opening login page");
                    return Command::handle_result(|it| async move {
                        it.app().authenticate().await;
                        Ok(())
                    });
                };

                let Some(me) = model.p2p.me.clone() else {
                    log::info!("Nearby service not available");
                    return Command::done()
                };

                let Some(signalling_key) = me.signalling_id.clone() else {
                    log::warn!("Missing signalling block");
                    return Command::done()
                };

                let Some(signalling_route) = me.signalling_route.clone() else {
                    log::warn!("Missing signalling route");
                    return Command::done()
                };

                Command::handle_result(move |it| async move {
                    it.app()
                        .start_p2p_transfer(
                            selected_resources,
                            password,
                            user,
                            shelf_id,
                            shelf_name,
                            signalling_key,
                            signalling_route
                        )
                        .await
                })
            }
            TransferEvent::NewTransferResource { shelf_id, resource } => {
                let Some(active_session) = model.transfer.get_active_p2p_send_session(shelf_id) else {
                    return Command::done()
                };

                let active_session_id = active_session.order_id;
                let id = TransferSessionId {
                    order_id: Some(active_session_id.to_string()),
                    transfer_type: Some(TransferType::Send { from_shelf_id: shelf_id })
                };

                let res = resource.clone();

                let mut commands = vec![];
                commands.push(Command::event(
                    TransferSessionModelEvent::Update(id.clone(), TransferSessionUpdateEvent::ResourceUpdate(res.clone())).into()
                ));

                if active_session.target.is_peer() {
                    commands.push(Command::new(move |it| {
                        let resource = res.clone();
                        async move {
                            it.notify_shell(CoreOperation::P2P(P2POperation::SendResourceNotification {
                                session_id: active_session_id,
                                resource
                            }));
                        }
                    }));
                }

                Command::all(commands)
            }
            TransferEvent::UpdateConnectionState { session_id, state } => {
                let session_id_key = TransferSessionId {
                    order_id: Some(session_id.to_string()),
                    transfer_type: Some(TransferType::Receive)
                };
                if let Some(session) = model.transfer.sessions.lookup_mut(&session_id_key) {
                    session.target.set_connection_state(state);
                }
                Command::render()
            }
            TransferEvent::PeerConnected { session_id, peer } => {
                let session_id_key = TransferSessionId {
                    order_id: Some(session_id.to_string()),
                    transfer_type: Some(TransferType::Receive)
                };
                if let Some(session) = model.transfer.sessions.lookup_mut(&session_id_key) {
                    if let TransferTarget::P2P {
                        from_peer,
                        connection_state,
                        ..
                    } = &mut session.target
                    {
                        *from_peer = Some(peer);
                        *connection_state = P2PConnectionState::Connected;
                    }
                }
                Command::render()
            }
            TransferEvent::PeerDisconnected { peer_id } => {
                model.transfer.sessions.retain(|s| {
                    if matches!(s.transfer_type, TransferType::Receive) {
                        if let TransferTarget::P2P { from_peer, .. } = &s.target {
                            if let Some(p) = from_peer {
                                return p.id != peer_id;
                            }
                        }
                    }
                    true
                });
                Command::render()
            }
            TransferEvent::ModelEvent(event) => {
                match event {
                    TransferSessionModelEvent::Update(session_id, action) => {
                        if let Some(session) = model.transfer.sessions.lookup_mut(&session_id) {
                            action.update(session);
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
            TransferEvent::FindSession { mut keywords } => {
                if let Ok(url) = url::Url::parse(&keywords) {
                    let Some(query) = url.query_pairs().find(|(key, _)| key == "session").map(|it| it.1.to_string()) else {
                        log::info!("Not found query key session");
                        return Command::done()
                    };

                    keywords = query;
                }

                Command::handle_result(|it| async move { it.app().find_transfer_session(keywords).await }).then_render()
            }
            TransferEvent::ViewSession { password, session_id, .. } => {
                let session_id = TransferSessionId {
                    order_id: Some(session_id.to_string()),
                    transfer_type: Some(TransferType::Receive)
                };

                let Some(session) = model.transfer.sessions.lookup_mut(&session_id) else {
                    log::info!("Session {:?} not found", session_id);
                    return Command::done()
                };

                if session.target.is_connection_failed() {
                    session.owner_disconnected();
                }

                let session = session.clone();
                Command::handle_result(move |it| async move { it.app().view_session(session, session_id, password).await })
                    .then_render()
            }
            TransferEvent::ReceivedViewSessionRequest {
                peer_id,
                request_id,
                order_id,
                password
            } => {
                let session_id = TransferSessionId {
                    order_id: Some(order_id.to_string()),
                    transfer_type: Some(TransferType::send_any())
                };

                log::info!("Received view session request {session_id:?}");

                let session = model.transfer.sessions.lookup(&session_id).cloned();
                let device = model.environment.device.clone();
                Command::handle_result(move |it| async move {
                    it.app().handle_view_session_request(peer_id, request_id, password, session, device).await
                })
            }
            TransferEvent::RequestSessionDetail {
                peer_id,
                order_id,
                password
            } => {
                let session_id = TransferSessionId {
                    order_id: Some(order_id.to_string()),
                    transfer_type: Some(TransferType::Receive)
                };

                Command::handle_result(move |it| async move {
                    it.app().request_session_detail(peer_id, session_id, order_id, password).await
                })
            }
            TransferEvent::ReceivedDownloadRequest {
                peer_id,
                session_order_id,
                resource_order_id,
                transfer_id
            } => {
                let session_id = TransferSessionId {
                    order_id: Some(session_order_id.to_string()),
                    transfer_type: Some(TransferType::send_any())
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
            TransferEvent::ResourceNotification {
                session_order_id,
                resource,
                peer_id: _
            } => {
                let session_id = TransferSessionId {
                    order_id: Some(session_order_id.to_string()),
                    transfer_type: Some(TransferType::Receive)
                };

                let Some(session) = model.transfer.sessions.lookup(&session_id) else {
                    log::warn!("Session {} not found for resource notification", session_order_id);
                    return Command::done();
                };

                let needs_recovery = session.connection_error.is_some() || session.target.is_connection_failed();
                let resource_order_id = resource.order_id;
                let resource_name = resource.name.clone();
                let resource_type = resource.r#type.clone();

                Command::handle_result(move |it| async move {
                    if needs_recovery {
                        log::info!(
                            "Session {} recovered from timeout, received resource notification",
                            session_order_id
                        );
                        it.update_model(TransferSessionModelEvent::Update(
                            session_id.clone(),
                            ConnectionRecovered.into()
                        ));
                    }
                    let mut generate_file_paths_request = HashMap::new();
                    generate_file_paths_request.insert(resource_order_id, (resource_name, resource_type));

                    let mut generated_saved_paths = it
                        .app()
                        .run(TransferSessionPersistentOperation::generate_resource_paths(
                            session_order_id,
                            generate_file_paths_request
                        ))
                        .await?;

                    let Some(generated_path) = generated_saved_paths.remove(&resource_order_id) else {
                        log::warn!("Failed to generate path for resource {}", resource_order_id);
                        return Ok(());
                    };

                    let mut updated_resource = resource;
                    updated_resource.name = match &updated_resource.r#type {
                        ResourceType::Folder => {
                            format!("{}.{}", updated_resource.name, generated_path.extension().unwrap_or_default())
                        }
                        _ => updated_resource.name.clone()
                    };

                    updated_resource.path = generated_path;

                    it.update_model(TransferSessionModelEvent::Update(
                        session_id,
                        TransferSessionUpdateEvent::ResourceUpdate(updated_resource)
                    ));

                    Ok(())
                })
            }
            TransferEvent::RequestDownloadResource {
                peer_id,
                session_order_id,
                resource_order_id
            } => {
                let id = TransferSessionId {
                    order_id: Some(session_order_id.to_string()),
                    transfer_type: Some(TransferType::Receive)
                };

                let Some(resource) = model
                    .transfer
                    .sessions
                    .lookup(&id)
                    .and_then(|s| s.resources.iter().find(|r| r.order_id == resource_order_id).cloned())
                else {
                    log::warn!("Resource not found in session: {}", resource_order_id);
                    return Command::done();
                };

                Command::handle_result(move |it| async move { it.app().request_download_resource(peer_id, id, resource).await })
            }
            TransferEvent::CancelResourceTransfer {
                session_id,
                transfer_type,
                resource_id
            } => {
                let id = TransferSessionId {
                    order_id: Some(session_id.to_string()),
                    transfer_type: Some(transfer_type)
                };

                let Some(session) = model.transfer.sessions.lookup(&id).cloned() else {
                    log::warn!("Session not found: {}", session_id);
                    return Command::done();
                };

                Command::handle_result(move |it| async move { it.app().cancel_resource_transfer(&session, resource_id).await })
            }
            TransferEvent::RequestDownloadAllResources { peer_id, session_order_id } => {
                let id = TransferSessionId {
                    order_id: Some(session_order_id.to_string()),
                    transfer_type: Some(TransferType::Receive)
                };

                let Some(session) = model.transfer.sessions.lookup(&id).cloned() else {
                    log::warn!("Session not found for download all: {}", session_order_id);
                    return Command::done();
                };

                Command::handle_result(move |it| async move { it.app().request_download_all_resources(peer_id, id, session).await })
            }
        }
    }

    fn view(&self, model: &AppModel) -> Self::ViewModel {
        let all_receive_sessions: Vec<_> = model
            .transfer
            .sessions
            .iter()
            .filter(|it| it.transfer_type == TransferType::Receive)
            .filter_map(|it| {
                let from_user = &it.from_user;
                let is_p2p = it.is_p2p_connected();

                let status = it.status();
                let is_loading = matches!(status, TransferSessionStatus::Initializing { .. });
                let loading_status = if let TransferSessionStatus::Initializing {
                    loading_state: Some(text), ..
                } = &status
                {
                    Some(text.clone())
                } else {
                    None
                };

                let error_message = if let TransferSessionStatus::Failed(msg) = &status {
                    Some(msg.clone())
                } else if let TransferSessionStatus::Initializing {
                    loading_error: Some(error),
                    ..
                } = &status
                {
                    Some(error.clone())
                } else {
                    None
                };

                let (sender_id, sender_avatar, sender_name, sender_description, alias, access_url, password, is_required_password) =
                    match &it.target {
                        TransferTarget::P2P { from_peer, .. } => {
                            let sender_id = from_peer.as_ref().map(|p| p.id.clone()).unwrap_or_else(|| from_user.id.to_string());
                            let alias = if !it.alias.is_empty() { Some(it.alias.clone()) } else { None };
                            (
                                sender_id,
                                from_user.avatar.clone(),
                                from_user.name.clone(),
                                it.description.clone().unwrap_or_default(),
                                alias,
                                None,
                                None,
                                it.is_required_password
                            )
                        }
                        TransferTarget::Internet { .. } => {
                            let access_url_ref = if !it.access_url.is_empty() {
                                Some(it.access_url.clone())
                            } else {
                                None
                            };
                            let alias = if !it.alias.is_empty() { Some(it.alias.clone()) } else { None };
                            let name = from_user.name.to_string();
                            (
                                from_user.id.to_string(),
                                from_user.avatar.clone(),
                                name,
                                "Public".to_string(),
                                alias,
                                access_url_ref,
                                it.password.clone(),
                                it.is_required_password
                            )
                        }
                    };

                let resources = it
                    .resources
                    .iter()
                    .filter_map(|resource| {
                        let progress = it.progress.iter().find(|p| p.resource_order_id == resource.order_id)?;

                        Some(ReceiveResourceViewModel {
                            model: SelectedResourceViewModel::from(resource),
                            completion: progress.percentage() as f32,
                            is_ready: is_p2p || progress.status.is_completed(),
                            is_completed: progress.status.is_completed(),
                            is_success: progress.is_success(),
                            error_message: match &progress.status {
                                TransferStatus::Fail(msg) => Some(msg.clone()),
                                _ => None
                            }
                        })
                    })
                    .collect();

                let download_all_resource = if is_p2p && !it.resources.is_empty() {
                    let download_all_progress = it.progress.iter().find(|p| p.resource_order_id == u64::MAX);

                    let model = if let Some(resource_all) = it.session_resource.as_ref() {
                        let mut m = SelectedResourceViewModel::from(resource_all);
                        m.name = format!("all-resources-{}.zip", it.alias);
                        m.order_id = u64::MAX.to_string();
                        m
                    } else {
                        SelectedResourceViewModel {
                            order_id: u64::MAX.to_string(),
                            name: format!("all-resources-{}.zip", it.alias),
                            size_gb: 0.0,
                            size_mb: 0.0,
                            size_kb: 0.0,
                            size_bytes: 0,
                            display_path: String::new(),
                            path: LocalResourcePath::RelativePath {
                                path: String::new(),
                                is_private: false
                            },
                            thumbnail_path: None,
                            r#type: ResourceType::File,
                            received_by_peers: Vec::new()
                        }
                    };

                    Some(if let Some(progress) = download_all_progress {
                        ReceiveResourceViewModel {
                            model,
                            completion: progress.percentage() as f32,
                            is_ready: progress.status.is_completed(),
                            is_completed: progress.status.is_completed(),
                            is_success: progress.is_success(),
                            error_message: match &progress.status {
                                TransferStatus::Fail(msg) => Some(msg.clone()),
                                _ => None
                            }
                        }
                    } else {
                        ReceiveResourceViewModel {
                            model,
                            completion: 0.0,
                            is_ready: true,
                            is_completed: false,
                            is_success: false,
                            error_message: None
                        }
                    })
                } else {
                    None
                };

                Some(ReceiveSessionViewModel {
                    is_cloud: it.target.is_public(),
                    is_scope_online: match &it.target {
                        TransferTarget::P2P { connection_state, .. } => {
                            matches!(connection_state, crate::entities::target::P2PConnectionState::Connected)
                        }
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
                    resources,
                    is_completed: it.is_completed(),
                    is_in_progress: matches!(status, TransferSessionStatus::InProgress { .. }),
                    display_download_speed: it.status().to_string(),
                    progress: it.total_progress(),
                    display_datetime: id_to_datetime(it.order_id).with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M").to_string(),
                    download_all_resource
                })
            })
            .collect();

        let mut sorted = all_receive_sessions;
        sorted.sort_by(|a, b| match (a.is_scope_online, b.is_scope_online) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => b.id.cmp(&a.id)
        });

        let received_session = sorted.iter().find(|s| !s.is_cloud).cloned();
        let received_cloud_session = sorted.iter().find(|s| s.is_cloud).cloned();

        let is_loading = received_session.as_ref().map(|s| s.is_loading).unwrap_or(false) ||
            received_cloud_session.as_ref().map(|s| s.is_loading).unwrap_or(false);

        let total_p2p_receive_progress = received_session.as_ref().filter(|s| s.progress > 0.0 && !s.is_completed).map(|s| s.progress);

        Self::ViewModel {
            transfer_method: model.transfer.selected_method.clone(),
            received_session,
            received_cloud_session,
            is_loading,
            cloud_sessions: model
                .transfer
                .sessions
                .iter()
                .filter(|it| matches!(it.transfer_type, TransferType::Send { .. }))
                .filter(|it| it.target.is_public())
                .filter_map(|it| match &it.target {
                    TransferTarget::Internet { to_emails } => {
                        let access_url = if !it.access_url.is_empty() {
                            Some(it.access_url.clone())
                        } else {
                            None
                        };
                        let status = it.status();
                        let shelf_id = match it.transfer_type {
                            TransferType::Send { from_shelf_id } => Some(from_shelf_id.to_string()),
                            _ => None
                        };

                        Some(CloudSession {
                            shelf_id,
                            display_download_speed: match access_url.is_none() {
                                true => "Initializing...".to_owned(),
                                false => match it.transfer_type {
                                    TransferType::Receive => "".to_owned(),
                                    TransferType::Send { .. } => status.to_string()
                                }
                            },
                            is_email: !to_emails.is_empty(),
                            password: it.password.clone(),
                            session_id: it.order_id.to_string(),
                            is_completed: it.is_completed(),
                            is_in_progress: matches!(status, TransferSessionStatus::InProgress { .. }),
                            progress: it.total_progress(),
                            access_url
                        })
                    }
                    _ => None
                })
                .collect(),
            p2p_sessions: model
                .transfer
                .sessions
                .iter()
                .filter(|it| matches!(it.transfer_type, TransferType::Send { .. }))
                .filter(|it| it.target.is_peer())
                .filter_map(|it| {
                    let access_url = if !it.access_url.is_empty() {
                        Some(it.access_url.clone())
                    } else {
                        None
                    };
                    let status = it.status();
                    let shelf_id = match it.transfer_type {
                        TransferType::Send { from_shelf_id } => Some(from_shelf_id.to_string()),
                        _ => None
                    };
                    Some(CloudSession {
                        shelf_id,
                        display_download_speed: status.to_string(),
                        is_email: false,
                        password: it.password.clone(),
                        session_id: it.order_id.to_string(),
                        is_completed: it.is_completed(),
                        is_in_progress: !it.is_completed(),
                        progress: it.total_progress(),
                        access_url
                    })
                })
                .collect(),
            total_p2p_receive_progress
        }
    }
}
