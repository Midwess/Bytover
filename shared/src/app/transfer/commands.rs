use crate::app::core::command::AppCommand;
use crate::app::core::extensions::CoreCommandContextUtils;
use crate::app::core::model_events::{PeerReceivedEvent, SessionLoadError, TransferSessionModelEvent, UpdateAction};
use crate::app::operations::dialog::{DialogOperation, MessageReason};
use crate::app::operations::p2p::P2POperation;
use crate::app::operations::persistent::TransferSessionPersistentOperation;
use crate::app::operations::rpc::RpcOperation;
use crate::app::operations::transfer::{TransferOperation, TransferOperationOutput};
use crate::app::operations::{CoreOperation, CoreOperationOutput};
use crate::app::p2p::module::P2PEvent;
use crate::app::transfer::module::TransferEvent;
use crate::entities::device::DeviceInfo;
use crate::entities::finding_scope::FindingScope;
use crate::entities::local_resource::LocalResource;
use crate::entities::peer::Peer;
use crate::entities::target::{P2PConnectionState, TransferTarget};
use crate::entities::transfer_session::{
    SessionResourceUpdate,
    TransferProgress,
    TransferSession,
    TransferSessionStatus,
    TransferStatus,
    TransferType
};
use crate::entities::user::User;
use crate::errors::CoreError;
use crate::repository::transfer_session::TransferSessionId;
use core_services::db::repository::abstraction::table::Table;
use core_services::utils::string::StringExt;
use n0_future::StreamExt;
use schema::devlog::bitbridge::PeerErrorsMessage;
use std::collections::HashMap;

pub const MAX_CONCURRENT_P2P_SESSIONS: usize = 5;

impl AppCommand {
    pub fn maybe_auto_view_p2p_session(&self, sessions: &[TransferSession]) -> Option<TransferEvent> {
        let p2p_receive_sessions: Vec<_> = sessions
            .iter()
            .filter(|s| matches!(s.transfer_type, TransferType::Receive) && s.target.is_peer())
            .collect();

        if p2p_receive_sessions.len() == 1 {
            let session = p2p_receive_sessions[0];
            Some(TransferEvent::ViewSession {
                password: session.password.clone(),
                session_id: session.order_id,
                transfer_type: TransferType::Receive
            })
        } else {
            None
        }
    }

    pub async fn load_transfer_sessions(&self) -> Result<(), CoreError> {
        let receive_sessions = self.run(TransferSessionPersistentOperation::get_all_received_sessions()).await?;

        for session in &receive_sessions {
            if let TransferTarget::P2P { ref scope, .. } = session.target {
                self.update_model(P2PEvent::AddFindingScope(scope.clone()));
            }
        }

        if let Some(auto_view_event) = self.maybe_auto_view_p2p_session(&receive_sessions) {
            self.update_model(auto_view_event);
        }

        let events = receive_sessions
            .into_iter()
            .map(|it| TransferEvent::ModelEvent(TransferSessionModelEvent::Add(it)))
            .collect::<Vec<_>>();
        self.update_model_series(events);

        Ok(())
    }

    pub async fn delete_session(&self, transfer_session: &TransferSession) -> Result<(), CoreError> {
        log::info!("Cancelling transfer: {:?}", transfer_session.order_id);

        let _ = self
            .run(TransferOperation::cancel_session(
                transfer_session.peer_id(),
                transfer_session.order_id
            ))
            .await;

        if let TransferTarget::P2P { ref scope, .. } = transfer_session.target {
            self.update_model(P2PEvent::RemoveFindingScope(scope.clone()));
        }

        let _ = self.run(TransferSessionPersistentOperation::remove(transfer_session.id())).await;
        self.update_model(TransferSessionModelEvent::Remove(transfer_session.id()));

        Ok(())
    }

    pub async fn upload(&self, session: TransferSession) -> Result<(), CoreError> {
        if session.resources.is_empty() {
            self.run(DialogOperation::toast("Please select at least one resource.".to_string())).await;
            return Ok(());
        }

        let TransferTarget::Internet { to_emails, .. } = &session.target else {
            return Ok(());
        };

        for email in to_emails.iter() {
            if !email.is_email() {
                self.run(DialogOperation::toast("Invalid email format".to_string())).await;
                return Ok(());
            }
        }

        let mut transfer_session = match self.run(TransferOperation::create_cloud_session(session.clone())).await {
            Err(err) => {
                self.run(DialogOperation::toast(format!("{err} please try again"))).await;
                return Ok(());
            }
            Ok(session) => session
        };

        let transfer_target_id = transfer_session.target.id();

        transfer_session.resources.sort_by(|a, b| a.size.cmp(&b.size));

        self.update_model(TransferSessionModelEvent::Add(transfer_session.clone()));

        log::info!("Begin transferring session to: {transfer_target_id:?}",);

        let mut stream = self.stream_from_shell(TransferOperation::SendSession(transfer_session.clone()).into());

        while let Some(output) = stream.next().await {
            match output {
                CoreOperationOutput::Transfer(transfer_output) => match transfer_output {
                    TransferOperationOutput::TransferResourceProgressUpdate(progress) => {
                        if progress.status.is_completed() {
                            log::info!(
                                "Resource {:?} completed with status {:?}",
                                progress.resource_order_id,
                                progress.status
                            );
                        }

                        transfer_session.update_progress(progress.clone());
                        self.update_model(TransferSessionModelEvent::Update(transfer_session.id(), progress.into()));
                    }
                    TransferOperationOutput::TransferCompleted(status) => {
                        if status == TransferSessionStatus::Canceled {
                            transfer_session.cancel();
                        }

                        break;
                    }
                    TransferOperationOutput::ThumbnailUpdated(thumbnail) => {
                        if let Some(resource) = transfer_session.resource_mut(thumbnail.resource_id).cloned() {
                            TransferSessionPersistentOperation::update_resource(transfer_session.id(), resource);
                        }

                        self.update_model(TransferSessionModelEvent::Update(transfer_session.id(), thumbnail.into()));
                    }
                    other => {
                        log::error!("Unexpected transfer output: {other:?}");
                        break;
                    }
                },
                CoreOperationOutput::Error(error) => {
                    log::error!("Error: {error:?}");
                    transfer_session.force_complete(format!("Connection error: {error:?}"));
                    break;
                }
                _ => {
                    continue;
                }
            }

            if transfer_session.is_completed() {
                break;
            }
        }

        log::info!("Complete transferring session");

        // We do not remove the public transfer since the user needs to see the information
        // after transfer completed.
        if transfer_session.is_success() && transfer_session.target.is_public() {
            return Ok(());
        }

        self.update_model(TransferSessionModelEvent::Remove(transfer_session.id()));

        Ok(())
    }

    pub async fn find_transfer_session(&self, keywords: String) -> Result<(), CoreError> {
        let session_overview = self.run(TransferOperation::find_transfer_session(keywords.clone())).await?;

        let Some(session) = session_overview else {
            log::info!("No session found");
            self.run(DialogOperation::message(
                "Not found".to_string(),
                MessageReason::FailedToFindPublicSession
            ))
            .await;
            return Ok(());
        };

        if let Err(e) = self.run(TransferSessionPersistentOperation::save(session.clone())).await {
            log::error!("Failed to save session: {e:?}");
        };

        if let TransferTarget::P2P { ref scope, .. } = session.target {
            self.update_model(P2PEvent::AddFindingScope(scope.clone()));
        }

        let should_auto_view = matches!(session.transfer_type, TransferType::Receive);
        let auto_view_event = if should_auto_view {
            Some(TransferEvent::ViewSession {
                password: session.password.clone(),
                session_id: session.order_id,
                transfer_type: TransferType::Receive
            })
        } else {
            None
        };

        self.update_model(TransferSessionModelEvent::Add(session));

        if let Some(event) = auto_view_event {
            self.update_model(event);
        }

        Ok(())
    }

    pub async fn view_public_session(
        &self,
        mut transfer_session: TransferSession,
        entered_password: Option<String>
    ) -> Result<(), CoreError> {
        let user_id = match &transfer_session.target {
            TransferTarget::Internet { .. } => {
                if let Some(entered_password) = entered_password {
                    transfer_session.password = Some(entered_password);
                };

                transfer_session.from_user.id
            }
            _ => {
                return Ok(());
            }
        };

        let password = transfer_session.password.clone();

        let session_order_id = transfer_session.order_id;
        let request = CoreOperation::Transfer(TransferOperation::SubscribeToPublicSessionTransferProgress {
            password,
            session_owner_user_id: user_id,
            session_order_id
        });

        let mut stream = self.stream_from_shell(request);
        while let Some(output) = stream.next().await {
            let transfer: TransferOperationOutput = match output.result() {
                Ok(output) => output,
                Err(err) => {
                    self.run(DialogOperation::message(
                        format!("{err}"),
                        MessageReason::FailedToLoadSession(session_order_id)
                    ))
                    .await;

                    return Err(err);
                }
            };

            match transfer {
                TransferOperationOutput::PublicTransferSessionUpdated((resources, progresses)) => {
                    let mut events = vec![];
                    for resource in resources {
                        events.push(TransferSessionModelEvent::Update(transfer_session.id(), resource.into()));
                    }

                    for progress in progresses {
                        events.push(TransferSessionModelEvent::Update(transfer_session.id(), progress.into()));
                    }

                    self.update_model_series(events);
                }
                TransferOperationOutput::SubscribeSessionEnded => {
                    break;
                }
                o => {
                    log::warn!("Unexpected transfer output: {o:?}");
                    continue;
                }
            };
        }

        Ok(())
    }

    pub async fn handle_view_session_request(
        &self,
        peer_id: String,
        request_id: String,
        password: Option<String>,
        session: Option<TransferSession>,
        device_info: Option<DeviceInfo>
    ) -> Result<(), CoreError> {
        use schema::devlog::bitbridge::{P2pTransferSessionMessage, PeerErrorsMessage};

        let Some(mut session) = session else {
            log::warn!("Failed to load session detail: session not found");
            self.run(P2POperation::send_session_detail_error(
                peer_id,
                request_id,
                CoreError::PeerRequestError(PeerErrorsMessage::SessionNotFound)
            ))
            .await?;
            return Ok(());
        };

        session.description = device_info.map(|it| it.name.to_string());

        if session.is_required_password {
            match (&session.password, &password) {
                (Some(expected), Some(provided)) if expected == provided => {
                    let proto_session = P2pTransferSessionMessage {
                        order_id: session.order_id,
                        description: session.description.clone(),
                        password_protected: true
                    };

                    self.run(P2POperation::send_session_detail(
                        peer_id,
                        request_id,
                        Some(proto_session),
                        Some(session.resources)
                    ))
                    .await?;
                }
                (Some(_), None) => {
                    let proto_session = P2pTransferSessionMessage {
                        order_id: session.order_id,
                        description: session.description.clone(),
                        password_protected: true
                    };

                    self.run(P2POperation::send_session_detail(
                        peer_id,
                        request_id,
                        Some(proto_session),
                        None
                    ))
                    .await?;
                }
                (Some(_), Some(_)) => {
                    log::warn!("Invalid password for session {}", session.order_id);
                    self.run(P2POperation::send_session_detail_error(
                        peer_id,
                        request_id,
                        CoreError::PeerRequestError(PeerErrorsMessage::InvalidPassword)
                    ))
                    .await?;
                }
                (None, _) => {
                    let proto_session = P2pTransferSessionMessage {
                        order_id: session.order_id,
                        description: session.description.clone(),
                        password_protected: false
                    };

                    self.run(P2POperation::send_session_detail(
                        peer_id,
                        request_id,
                        Some(proto_session),
                        Some(session.resources)
                    ))
                    .await?;
                }
            }
        } else {
            let proto_session = P2pTransferSessionMessage {
                order_id: session.order_id,
                description: session.description.clone(),
                password_protected: false
            };

            self.run(P2POperation::send_session_detail(
                peer_id,
                request_id,
                Some(proto_session),
                Some(session.resources)
            ))
            .await?;
        }

        Ok(())
    }

    pub async fn request_session_detail(
        &self,
        peer_id: String,
        session_id: TransferSessionId,
        order_id: u64,
        password: Option<String>
    ) -> Result<(), CoreError> {
        let mut stream = self.stream_from_shell(
            P2POperation::ViewSessionDetail {
                peer_id,
                order_id,
                password
            }
            .into()
        );

        while let Some(output) = stream.next().await {
            match output {
                CoreOperationOutput::Transfer(TransferOperationOutput::SessionDetailReceived(proto_session)) => {
                    log::info!(
                        "Received session detail for order_id {}: description={:?}, password_protected={}",
                        proto_session.order_id,
                        proto_session.description,
                        proto_session.password_protected
                    );

                    self.update_model(TransferSessionModelEvent::Update(session_id.clone(), proto_session.into()));
                    break;
                }
                CoreOperationOutput::Error(e) => {
                    let msg = match &e {
                        CoreError::PeerRequestError(PeerErrorsMessage::SessionNotFound) => "Session not found".to_string(),
                        CoreError::PeerRequestError(PeerErrorsMessage::InvalidPassword) => "Invalid password".to_string(),
                        _ => format!("Failed to load session detail: {e:?}")
                    };

                    log::error!("Error receiving session detail: {:?}", e);

                    if matches!(e, CoreError::PeerRequestError(PeerErrorsMessage::SessionNotFound)) {
                        log::info!("Deleting session {:?} from database as it was not found", session_id);
                        let _ = self.run(TransferSessionPersistentOperation::remove(session_id.clone())).await;
                        self.update_model(TransferSessionModelEvent::Remove(session_id.clone()));
                    } else {
                        self.update_model(TransferSessionModelEvent::Update(
                            session_id.clone(),
                            SessionLoadError(msg).into()
                        ));
                    }

                    return Err(e);
                }
                _ => continue
            }
        }

        Ok(())
    }

    pub async fn handle_download_request(
        &self,
        peer: Option<Peer>,
        session_id: u64,
        transfer_id: u16,
        resource: Option<LocalResource>
    ) -> Result<(), CoreError> {
        let Some(resource) = resource else {
            log::error!("Resource not found for download request");
            return Ok(());
        };

        let Some(peer) = peer else {
            log::error!("Peer not found for download request");
            return Ok(());
        };

        let resource_order_id = resource.order_id;
        let peer_id = peer.id.clone();
        let result = self
            .run(P2POperation::stream_resource_to_peer(
                peer_id.clone(),
                session_id,
                transfer_id,
                resource
            ))
            .await;

        match result {
            Ok(()) => {
                let session_id_obj = TransferSessionId {
                    order_id: Some(session_id.to_string()),
                    transfer_type: Some(TransferType::send_any())
                };
                self.update_model(TransferSessionModelEvent::Update(
                    session_id_obj,
                    PeerReceivedEvent {
                        resource_order_id,
                        peer
                    }
                    .into()
                ));
            }
            Err(e) => {
                log::error!("Failed to stream resource to peer: {e:?}");
            }
        }

        Ok(())
    }

    pub async fn request_download_resource(
        &self,
        peer_id: String,
        session_id: TransferSessionId,
        resource: LocalResource
    ) -> Result<(), CoreError> {
        let mut progress = TransferProgress::new(resource.order_id, resource.size, TransferType::Receive);

        self.update_model(TransferSessionModelEvent::Update(session_id.clone(), progress.clone().into()));

        let mut stream = self.stream_from_shell(
            P2POperation::DownloadResource {
                peer_id,
                session_id: session_id.order_id.clone().unwrap_or_default().parse().unwrap_or_default(),
                resource,
                progress: progress.clone()
            }
            .into()
        );
        while let Some(output) = stream.next().await {
            match output {
                CoreOperationOutput::Transfer(TransferOperationOutput::TransferResourceProgressUpdate(new_progress)) => {
                    progress = new_progress;
                    self.update_model(TransferSessionModelEvent::Update(session_id.clone(), progress.clone().into()));
                    if progress.is_completed() {
                        log::info!("Resource download completed with progress {progress:?}");
                        break;
                    }
                }
                CoreOperationOutput::Error(e) => {
                    log::info!("Download resource error: {e:?}");
                    progress.fail(e.to_string());
                    self.update_model(TransferSessionModelEvent::Update(session_id.clone(), progress.clone().into()));
                    break;
                }
                _ => continue
            }
        }

        Ok(())
    }

    pub async fn cancel_resource_transfer(&self, session: &TransferSession, resource_id: Option<u64>) -> Result<(), CoreError> {
        if !session.target.is_peer() {
            log::warn!("Cancel resource transfer is only supported for P2P sessions");
            return Ok(());
        }

        match session.transfer_type {
            TransferType::Send { .. } => {
                log::info!("Broadcasting cancel for session {} to all receivers", session.order_id);
                self.run(P2POperation::broadcast_cancel_session(session.order_id, resource_id)).await?;
            }
            TransferType::Receive => {
                let peer_id = match session.peer_id() {
                    Some(id) => id,
                    None => {
                        log::error!("P2P session has no peer_id");
                        return Ok(());
                    }
                };

                if let Some(resource_id) = resource_id {
                    log::info!("Cancelling resource {} for session {}", resource_id, session.order_id);
                    self.run(P2POperation::cancel_resource(peer_id, session.order_id, resource_id)).await?;
                } else {
                    log::info!("Cancelling session {} from receiver", session.order_id);
                    self.run(TransferOperation::cancel_session(Some(peer_id), session.order_id)).await?;
                }
            }
        }

        Ok(())
    }

    pub async fn request_download_all_resources(
        &self,
        peer_id: String,
        session_id: TransferSessionId,
        mut session: TransferSession
    ) -> Result<(), CoreError> {
        use crate::entities::local_resource::ResourceType;
        use crate::repository::transfer_session::ZipDownloadPaths;

        let resource_names: HashMap<u64, String> = session.resources.iter().map(|r| (r.order_id, r.name.clone())).collect();

        let zip_paths: ZipDownloadPaths = self
            .run(TransferSessionPersistentOperation::generate_zip_download_paths(
                session.order_id,
                resource_names
            ))
            .await?;

        for resource in &mut session.resources {
            if let Some(path) = zip_paths.resource_paths.get(&resource.order_id) {
                resource.path = path.clone();
            }
        }

        let total_size: u64 = session.resources.iter().map(|r| r.size).sum();
        let session_resource = LocalResource {
            order_id: u64::MAX,
            name: format!("{}.zip", session.order_id),
            size: total_size,
            path: zip_paths.session_path.clone(),
            r#type: ResourceType::File,
            thumbnail_path: None,
            shelf_id: 0
        };

        let mut aggregate_progress = TransferProgress::new(u64::MAX, total_size, TransferType::Receive);
        session.session_resource = Some(session_resource.clone());
        session.update_progress(aggregate_progress.clone());

        self.update_model(TransferSessionModelEvent::Update(
            session_id.clone(),
            SessionResourceUpdate(session_resource.clone()).into()
        ));

        self.update_model(TransferSessionModelEvent::Update(
            session_id.clone(),
            aggregate_progress.clone().into()
        ));

        let mut stream = self.stream_from_shell(
            P2POperation::DownloadAllResources {
                peer_id,
                session_id: session.order_id,
                session_path: session_resource,
                resources: session.resources.clone(),
                aggregate_progress: aggregate_progress.clone()
            }
            .into()
        );

        let mut resource_progress_map: HashMap<u64, u64> = HashMap::new();

        while let Some(output) = stream.next().await {
            match output {
                CoreOperationOutput::Transfer(TransferOperationOutput::TransferResourceProgressUpdate(progress)) => {
                    if progress.is_failed() {
                        log::warn!("Resource {} failed, cancelling download all", progress.resource_order_id);
                        aggregate_progress.fail(format!("Resource {} failed", progress.resource_order_id));
                        self.update_model(TransferSessionModelEvent::Update(
                            session_id.clone(),
                            aggregate_progress.clone().into()
                        ));
                        break;
                    }

                    if progress.is_canceled() {
                        log::warn!("Resource {} cancelled, cancelling download all", progress.resource_order_id);
                        aggregate_progress.status = TransferStatus::Canceled;
                        self.update_model(TransferSessionModelEvent::Update(
                            session_id.clone(),
                            aggregate_progress.clone().into()
                        ));
                        break;
                    }

                    if progress.resource_order_id == aggregate_progress.resource_order_id {
                        log::info!("Received session progress {:?}", progress);
                        if progress.is_completed() {
                            aggregate_progress = progress;
                        }
                    }
                    else {
                        let current = resource_progress_map.entry(progress.resource_order_id).or_insert(progress.total_bytes());
                        *current = (*current).max(progress.total_bytes());

                        let total_downloaded: u64 = resource_progress_map.values().sum();
                        let bytes_delta = total_downloaded.saturating_sub(aggregate_progress.total_bytes());
                        aggregate_progress.update_progress(bytes_delta);
                    }

                    self.update_model(TransferSessionModelEvent::Update(
                        session_id.clone(),
                        aggregate_progress.clone().into()
                    ));

                    if aggregate_progress.is_completed() {
                        log::info!("All resources download completed");
                        break;
                    }
                }
                CoreOperationOutput::Error(e) => {
                    log::info!("Download all resources error: {e:?}");
                    aggregate_progress.fail(e.to_string());
                    self.update_model(TransferSessionModelEvent::Update(
                        session_id.clone(),
                        aggregate_progress.clone().into()
                    ));
                    break;
                }
                _ => continue
            }
        }

        Ok(())
    }

    pub async fn view_session(
        &self,
        session: TransferSession,
        session_id: TransferSessionId,
        password: Option<String>
    ) -> Result<(), CoreError> {
        use crate::app::AppEvent;

        match &session.target {
            TransferTarget::P2P {
                connection_state,
                scope,
                from_peer,
                ..
            } => {
                let should_request = match connection_state {
                    P2PConnectionState::NotConnected | P2PConnectionState::Failed(_) => false,
                    P2PConnectionState::Connected => session.resources.is_empty(),
                    P2PConnectionState::Connecting => false
                };

                if !should_request {
                    return Ok(());
                }

                if from_peer.is_none() {
                    self.update_model(AppEvent::P2P(P2PEvent::AddFindingScope(scope.clone())));
                    return Ok(());
                }

                let peer_id = from_peer.as_ref().unwrap().id().to_string();
                self.request_session_detail(peer_id, session_id, session.order_id, password).await
            }
            TransferTarget::Internet { .. } => self.view_public_session(session, password).await
        }
    }

    pub async fn start_p2p_transfer(
        &self,
        selected_resources: Vec<LocalResource>,
        password: Option<String>,
        user: User,
        from_shelf_id: u64,
        shelf_name: String
    ) -> Result<(), CoreError> {
        let p2p_session = self.run(RpcOperation::create_p2p_session(shelf_name)).await?;

        let mut session = TransferSession::p2p(
            selected_resources,
            password,
            p2p_session.signalling_room_id.clone(),
            p2p_session.signalling_scope.clone(),
            p2p_session.alias.clone(),
            p2p_session.access_url.clone(),
            p2p_session.session_id,
            from_shelf_id
        );

        let scope = FindingScope::new(&p2p_session.signalling_room_id);
        self.update_model(P2PEvent::AddFindingScope(scope));

        session.from_user = user;
        self.update_model(TransferSessionModelEvent::Add(session.clone()));

        Ok(())
    }
}
