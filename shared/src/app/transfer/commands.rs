use std::collections::HashMap;

use core_services::db::repository::abstraction::table::Table;
use devlog_sdk::distributed_id::gen_id_sync;
use futures_util::StreamExt;
use schema::devlog::bitbridge::{ResourceTypeMessage, TransferSessionMessage};

use crate::app::core::command::AppCommand;
use crate::app::core::extensions::CoreCommandContextUtils;
use crate::app::core::model_events::TransferSessionModelEvent;
use crate::app::operations::dialog::{DialogOperation, MessageReason};
use crate::app::operations::persistent::TransferSessionPersistentOperation;
use crate::app::operations::transfer::{TransferOperation, TransferOperationOutput};
use crate::app::operations::{CoreOperation, CoreOperationOutput};
use crate::app::transfer::module::TransferEvent;
use crate::entities::local_resource::{LocalResource, ResourceType};
use crate::entities::peer::Peer;
use crate::entities::target::TransferTarget;
use crate::entities::transfer_session::{TransferSession, TransferSessionStatus};
use crate::entities::user::User;

impl AppCommand {
    pub async fn load_transfer_sessions(&self) {
        let receive_sessions = self.run(TransferSessionPersistentOperation::get_all_received_sessions()).await;
        let events = receive_sessions
            .into_iter()
            .map(|it| TransferEvent::ModelEvent(TransferSessionModelEvent::Add(it)))
            .collect::<Vec<_>>();
        self.update_model_series(events);
    }

    pub async fn delete_session(&self, transfer_session: TransferSession) {
        if !transfer_session.is_completed() {
            log::info!("Cancelling transfer: {:?}", transfer_session.order_id);

            self.update_model(TransferSessionModelEvent::Remove(transfer_session.id()));

            if let Err(error) = self
                .run(TransferOperation::cancel_session(
                    transfer_session.peer_id(),
                    transfer_session.order_id
                ))
                .await
            {
                log::error!("Failed to cancel transfer: {error:?}");
            }
        }

        let _ = self.run(TransferSessionPersistentOperation::remove(transfer_session.id())).await;
    }

    pub async fn transfer(&self, user: User, selected_resources: Vec<LocalResource>, transfer_target: TransferTarget) {
        if selected_resources.is_empty() {
            self.run(DialogOperation::toast("Please select at least one resource.".to_string())).await;
            return;
        }

        let transfer_target_id = transfer_target.id();
        let mut transfer_session = match transfer_target {
            TransferTarget::Internet { password, to_emails, .. } => {
                for email in to_emails.iter() {
                    let has_at = email.contains('@');
                    let has_dot = email.contains('.');
                    let has_valid_length = email.len() >= 3;
                    let parts: Vec<&str> = email.split('@').collect();
                    let valid_parts = parts.len() == 2 && !parts[0].is_empty() && !parts[1].is_empty();
                    let domain_parts: Vec<&str> = parts.get(1).unwrap_or(&"").split('.').collect();
                    let valid_domain = domain_parts.len() >= 2 && domain_parts.iter().all(|p| !p.is_empty());

                    if !(has_at && has_dot && has_valid_length && valid_parts && valid_domain) {
                        self.run(DialogOperation::toast("Invalid email format".to_string())).await;
                        return;
                    }
                }

                let session = TransferSession::public(user, password, selected_resources, to_emails);
                let result = match self.run(TransferOperation::create_cloud_session(session)).await {
                    Err(err) => {
                        self.run(DialogOperation::toast(format!("{err} please try again"))).await;
                        return;
                    }
                    Ok(session) => session
                };

                result
            }
            TransferTarget::Nearby(_) => {
                let order_id = gen_id_sync();
                let result = TransferSession::send(order_id, selected_resources, transfer_target).await;

                result
            }
        };

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
                        TransferSessionPersistentOperation::update_progresses(transfer_session.order_id, vec![progress.clone()]);
                        let id = transfer_session.id();
                        self.update_model(TransferSessionModelEvent::Update(id, progress.into()));
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
                    self.run(DialogOperation::toast(format!("{error}"))).await;
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
            return;
        }

        let _ = self.run(TransferSessionPersistentOperation::remove(transfer_session.id())).await;

        self.update_model(TransferSessionModelEvent::Remove(transfer_session.id()));
    }

    pub async fn accept_session(&self, remote_session: TransferSessionMessage, peer: Peer) {
        let peer_id = peer.id();
        let (generate_file_paths_request, _generate_thumbnail_paths_request) = {
            let mut result = HashMap::new();
            let mut thumbnail_paths = HashMap::new();
            for resource in remote_session.resources.iter() {
                result.insert(resource.order_id, resource.name.clone());
                if resource.is_thumbnail_included {
                    thumbnail_paths.insert(resource.order_id, resource.name.clone());
                }
            }

            (result, thumbnail_paths)
        };

        let mut generated_thumbnails_paths = self
            .run(TransferSessionPersistentOperation::generate_thumbnail_paths(
                Some(remote_session.order_id),
                generate_file_paths_request.keys().copied().collect()
            ))
            .await;

        let mut generated_saved_paths = self
            .run(TransferSessionPersistentOperation::generate_resource_paths(
                remote_session.order_id,
                generate_file_paths_request
            ))
            .await;

        let mut resources = vec![];
        for resource_request in remote_session.resources {
            let order_id = resource_request.order_id;
            let Some(saved_path) = generated_saved_paths.remove(&order_id) else {
                continue;
            };

            let generated_thumbnail_path = generated_thumbnails_paths.remove(&order_id);

            resources.push(LocalResource {
                path: saved_path,
                thumbnail_path: generated_thumbnail_path,
                r#type: ResourceType::from(
                    ResourceTypeMessage::try_from(resource_request.r#type).unwrap_or(ResourceTypeMessage::File)
                ),
                name: resource_request.name.clone(),
                size: resource_request.size as u64,
                order_id: resource_request.order_id
            });
        }

        let response_transfer_session = TransferSession::answer(remote_session.order_id, resources, TransferTarget::Nearby(peer));

        let mut transfer_session = response_transfer_session.clone();
        // The thumbnail path at this point is not valid, since we are not received any thumbnail yet.
        transfer_session.resources.iter_mut().for_each(|r| r.thumbnail_path = None);
        self.update_model(TransferSessionModelEvent::Add(transfer_session.clone()));

        let response = CoreOperation::Transfer(TransferOperation::AnswerSessionRequest {
            peer_id: peer_id.to_string(),
            session: Some(response_transfer_session),
            session_id: transfer_session.order_id
        });

        let mut stream = self.stream_from_shell(response);
        while let Some(transfer_output) = stream.next().await {
            match transfer_output {
                CoreOperationOutput::Transfer(transfer_output) => match transfer_output {
                    TransferOperationOutput::TransferResourceProgressUpdate(progress) => {
                        TransferSessionPersistentOperation::update_progresses(transfer_session.order_id, vec![progress.clone()]);
                        self.update_model(TransferSessionModelEvent::Update(transfer_session.id(), progress.into()));
                    }
                    TransferOperationOutput::TransferCompleted(status) => {
                        if matches!(
                            status,
                            TransferSessionStatus::InProgress { .. } | TransferSessionStatus::Initializing
                        ) {
                            transfer_session.cancel();
                        }

                        log::info!(target: "transfer", "Transfer session completed with status {:?}", transfer_session.status());
                        break;
                    }
                    TransferOperationOutput::ThumbnailUpdated(event) => {
                        let resource = transfer_session.resource_mut(event.resource_id).unwrap();
                        resource.thumbnail_path = Some(event.path.clone());
                        let resource = resource.clone();

                        TransferSessionPersistentOperation::update_resource(transfer_session.id(), resource);
                        self.update_model(TransferSessionModelEvent::Update(transfer_session.id(), event.into()));
                    }
                    _ => {
                        continue;
                    }
                },
                CoreOperationOutput::Error(error) => {
                    transfer_session.force_complete(format!("Connection error: {error:?}"));
                    log::error!(target: "transfer", "Connection error: {error:?}");
                    break;
                }
                _ => {
                    continue;
                }
            }

            if transfer_session.is_completed() {
                log::info!(target: "transfer", "Transfer session completed");
                break;
            }
        }

        // Remove the session and add the new session
        if matches!(transfer_session.status(), TransferSessionStatus::Success) {
            self.run(TransferSessionPersistentOperation::remove(transfer_session.id())).await;
            self.run(TransferSessionPersistentOperation::save(transfer_session.clone())).await;
            self.update_model_series(vec![
                TransferSessionModelEvent::Remove(transfer_session.id()),
                TransferSessionModelEvent::Add(transfer_session.clone()),
            ]);
        } else {
            self.run(TransferSessionPersistentOperation::remove(transfer_session.id())).await;
            DialogOperation::toast("Transfer session canceled".to_string());
        }
    }

    pub async fn find_transfer_session(&self, keywords: String) {
        let session_overview = match self.run(TransferOperation::find_transfer_session(keywords)).await {
            Err(e) => {
                log::error!(target: "transfer", "Failed to find transfer session: {e:?}");
                self.run(DialogOperation::toast(format!("{e}"))).await;
                return;
            }
            Ok(session_overview) => session_overview
        };

        let Some(session) = session_overview else {
            self.run(DialogOperation::message(
                "Not found 🤔".to_owned(),
                MessageReason::FailedToFindPublicSession
            ))
            .await;
            return;
        };

        self.update_model(TransferSessionModelEvent::Add(session));
    }

    pub async fn view_public_session(&self, mut transfer_session: TransferSession, entered_password: Option<String>) {
        let (password, user_id) = match &mut transfer_session.target {
            TransferTarget::Internet { password, from_user, .. } => {
                if let Some(entered_password) = entered_password {
                    password.replace(entered_password);
                };

                (password.clone(), from_user.id)
            }
            _ => {
                return;
            }
        };

        let request = CoreOperation::Transfer(TransferOperation::SubscribeToPublicSessionTransferProgress {
            password,
            session_owner_user_id: user_id,
            session_order_id: transfer_session.order_id
        });

        let mut stream = self.stream_from_shell(request);
        while let Some(output) = stream.next().await {
            match output {
                CoreOperationOutput::Transfer(transfer) => match transfer {
                    TransferOperationOutput::PublicTransferSessionUpdated((resources, progresses)) => {
                        for resource in resources {
                            self.update_model(TransferSessionModelEvent::Update(transfer_session.id(), resource.into()));
                        }

                        for progress in progresses {
                            self.update_model(TransferSessionModelEvent::Update(transfer_session.id(), progress.into()));
                        }
                    }
                    TransferOperationOutput::SubscribeSessionEnded => {
                        break;
                    }
                    _ => return
                },
                CoreOperationOutput::Error(error) => {
                    self.run(DialogOperation::toast(format!("{error}"))).await;
                    return;
                }
                _ => return
            };
        }
    }
}
