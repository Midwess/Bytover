use core_services::db::repository::abstraction::table::Table;
use n0_future::StreamExt;
use core_services::utils::string::StringExt;

use crate::app::core::command::AppCommand;
use crate::app::core::extensions::CoreCommandContextUtils;
use crate::app::core::model_events::{TransferSessionModelEvent, UpdateAction};
use crate::app::operations::dialog::{DialogOperation, MessageReason};
use crate::app::operations::p2p::{P2POperation, P2PSessionOverview};
use crate::app::operations::persistent::TransferSessionPersistentOperation;
use crate::app::operations::transfer::{TransferOperation, TransferOperationOutput};
use crate::app::operations::{CoreOperation, CoreOperationOutput};
use crate::app::transfer::module::TransferEvent;
use crate::entities::local_resource::LocalResource;
use crate::entities::target::TransferTarget;
use crate::entities::transfer_session::{TransferProgress, TransferSession, TransferSessionStatus, TransferType};
use crate::entities::user::User;
use crate::errors::CoreError;
use crate::repository::transfer_session::TransferSessionId;

impl AppCommand {
    pub async fn load_transfer_sessions(&self) -> Result<(), CoreError> {
        let receive_sessions = self.run(TransferSessionPersistentOperation::get_all_received_sessions()).await?;
        let events = receive_sessions
            .into_iter()
            .map(|it| TransferEvent::ModelEvent(TransferSessionModelEvent::Add(it)))
            .collect::<Vec<_>>();
        self.update_model_series(events);

        Ok(())
    }

    pub async fn delete_session(&self, transfer_session: &TransferSession) -> Result<(), CoreError> {
        log::info!("Cancelling transfer: {:?}", transfer_session.order_id);

        let _ = self.run(TransferOperation::cancel_session(
            transfer_session.peer_id(),
            transfer_session.order_id
        ))
            .await;

        let _ = self.run(TransferSessionPersistentOperation::remove(transfer_session.id())).await;
        self.update_model(TransferSessionModelEvent::Remove(transfer_session.id()));

        Ok(())
    }

    pub async fn upload(
        &self,
        user: User,
        selected_resources: Vec<LocalResource>,
        transfer_target: TransferTarget
    ) -> Result<(), CoreError> {
        if selected_resources.is_empty() {
            self.run(DialogOperation::toast("Please select at least one resource.".to_string())).await;
            return Ok(());
        }

        let TransferTarget::Internet { password, to_emails, .. } = transfer_target.clone() else {
            return Ok(());
        };

        for email in to_emails.iter() {
            if !email.is_email() {
                self.run(DialogOperation::toast("Invalid email format".to_string())).await;
                return Ok(());
            }
        }

        let session = TransferSession::public(user, password, selected_resources, to_emails);
        let mut transfer_session = match self.run(TransferOperation::create_cloud_session(session)).await {
            Err(err) => {
                self.run(DialogOperation::toast(format!("{err} please try again"))).await;
                return Ok(());
            }
            Ok(session) => session
        };

        let transfer_target_id = transfer_target.id();

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

                        progress.clone().update(&mut transfer_session);
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
                    // self.run(DialogOperation::toast(format!("{error}"))).await;
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
        let session_overview = self.run(TransferOperation::find_transfer_session(keywords)).await?;

        let Some(session) = session_overview else {
            log::info!("No session found");
            self.run(DialogOperation::message(
                "Not found 🤔".to_owned(),
                MessageReason::FailedToFindPublicSession
            ))
            .await;
            return Ok(());
        };

        if let Err(e) = self.run(TransferSessionPersistentOperation::save(session.clone())).await {
            log::error!("Failed to save session: {e:?}");
        };

        self.update_model(TransferSessionModelEvent::Add(session));
        Ok(())
    }

    pub async fn view_public_session(
        &self,
        mut transfer_session: TransferSession,
        entered_password: Option<String>
    ) -> Result<(), CoreError> {
        let (password, user_id) = match &mut transfer_session.target {
            TransferTarget::Internet { password, from_user, .. } => {
                if let Some(entered_password) = entered_password {
                    password.replace(entered_password);
                };

                (password.clone(), from_user.id)
            }
            _ => {
                return Ok(());
            }
        };

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

    pub async fn notify_peer_sessions(
        &self,
        peer_id: String,
        sessions: Vec<TransferSession>
    ) -> Result<(), CoreError> {
        self.run(P2POperation::send_sessions_notification(peer_id, sessions)).await?;
        Ok(())
    }

    pub async fn handle_view_session_request(
        &self,
        peer_id: String,
        request_id: String,
        password: Option<String>,
        session: Option<TransferSession>
    ) -> Result<(), CoreError> {
        let Some(session) = session else {
           return Ok(());
        };

        let is_password_valid = match &session.target {
            TransferTarget::P2P { password: session_password, is_required_password, .. } => {
                if *is_required_password {
                    match (session_password, &password) {
                        (Some(expected), Some(provided)) => expected == provided,
                        (Some(_), None) => false,
                        (None, _) => true
                    }
                } else {
                    true
                }
            }
            _ => false
        };

        if !is_password_valid {
            return Ok(());
        }

        self.run(P2POperation::send_session_detail(
            peer_id,
            request_id,
            session
        )).await?;

        Ok(())
    }

    pub async fn request_session_detail(
        &self,
        peer_id: String,
        order_id: u64,
        password: Option<String>
    ) -> Result<(), CoreError> {
        let mut stream= self.stream_from_shell(P2POperation::ViewSessionDetail {
            peer_id,
            order_id,
            password
        }.into());

        let mut session_id = None;
        while let Some(output) = stream.next().await {
            match output {
                CoreOperationOutput::TransferSession(session) => {
                    session_id.replace(session.id());
                    self.update_model(TransferSessionModelEvent::Remove(session.id()));
                    self.update_model(TransferSessionModelEvent::Add(session));
                }
                CoreOperationOutput::LocalResource(resource) => {
                    if let Some(session_id) = session_id.clone() {
                        self.update_model(TransferSessionModelEvent::Update(session_id, resource.into()));
                    }
                }
                CoreOperationOutput::Error(err) => {
                    log::error!("Failed to load session detail: {err:?}");
                    break;
                }
                _ => continue
            }
        }

        Ok(())
    }

    pub async fn handle_download_request(
        &self,
        peer_id: String,
        session_id: u64,
        transfer_id: u16,
        resource: Option<LocalResource>
    ) -> Result<(), CoreError> {
        let Some(resource) = resource else {
            log::error!("Resource not found for download request");
            return Ok(());
        };

        self.run(P2POperation::stream_resource_to_peer(peer_id, session_id, transfer_id, resource)).await?;
        Ok(())
    }

    pub async fn request_download_resource(
        &self,
        peer_id: String,
        session_id: TransferSessionId,
        resource: LocalResource,
    ) -> Result<(), CoreError> {
        let mut progress = TransferProgress::new(
            resource.order_id,
            resource.size,
            TransferType::Receive
        );

        self.update_model(TransferSessionModelEvent::Update(session_id.clone(), progress.clone().into()));

        let mut stream = self.stream_from_shell(P2POperation::DownloadResource { peer_id, session_id: session_id.order_id.clone().unwrap_or_default().parse().unwrap_or_default(), resource, progress: progress.clone() }.into());
        while let Some(output) = stream.next().await {
            match output {
                CoreOperationOutput::Transfer(TransferOperationOutput::TransferResourceProgressUpdate(new_progress)) => {
                    progress = progress;
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
}
