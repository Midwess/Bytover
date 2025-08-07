use std::collections::HashMap;
use std::sync::OnceLock;

use futures_util::StreamExt;
use schema::devlog::bitbridge::{ResourceTypeMessage, TransferSessionMessage};

use super::session::TransferSession;
use super::target::TransferTarget;
use crate::app::core_utils::CoreCommandContextUtils;
use crate::app::file_system::file::{LocalResource, ResourceType};
use crate::app::modules::transfer::TransferEvent;
use crate::app::operations::dialog::{DialogOperation, MessageReason};
use crate::app::operations::persistent::{PersistentOperation, TransferSessionPersistentOperation};
use crate::app::operations::transfer::{TransferOperation, TransferOperationOutput};
use crate::app::operations::{CoreOperation, CoreOperationOutput};
use crate::app::transfer::session::TransferSessionStatus;
use crate::app::{AppCommandContext, AppEvent};
use crate::entities::peer::Peer;
use crate::entities::user::User;

pub struct TransferService {}

impl Default for TransferService {
    fn default() -> Self {
        Self::new()
    }
}

impl TransferService {
    pub fn new() -> Self {
        Self {}
    }

    pub fn instance() -> &'static TransferService {
        static INSTANCE: OnceLock<TransferService> = OnceLock::new();
        INSTANCE.get_or_init(TransferService::new)
    }

    pub async fn load_transfer_sessions(&self, cmd: AppCommandContext) {
        log::info!(target: "transfer", "Loading transfer sessions");

        let receive_sessions = TransferSessionPersistentOperation::get_all_received_sessions().into_future(cmd.clone()).await;

        let event = AppEvent::Transfer(TransferEvent::UpdateTransferSessions {
            loaded: receive_sessions,
            added: vec![],
            removed: vec![],
            updated: vec![]
        });

        cmd.notify_event(event);
    }

    pub async fn delete_session(&self, mut transfer_session: TransferSession, cmd: AppCommandContext) {
        if !transfer_session.is_completed() {
            log::info!(target: "transfer", "Cancelling transfer: {:?}", transfer_session.order_id);

            cmd.notify_event(AppEvent::Transfer(TransferEvent::UpdateTransferSessions {
                loaded: vec![],
                added: vec![],
                removed: vec![(transfer_session.order_id, transfer_session.transfer_type.clone())],
                updated: vec![]
            }));

            if let Err(error) = TransferOperation::cancel_session(transfer_session.peer_id(), transfer_session.order_id)
                .into_future(cmd.clone())
                .await
            {
                log::error!(target: "transfer", "Failed to cancel transfer: {error:?}");
            }
        }

        let _ = TransferSessionPersistentOperation::remove(
            transfer_session.order_id,
            transfer_session.transfer_type.clone()
        ).into_future(cmd.clone()).await;

        cmd.notify_event(AppEvent::Transfer(TransferEvent::UpdateTransferSessions {
            loaded: vec![],
            added: vec![],
            removed: vec![(transfer_session.order_id, transfer_session.transfer_type.clone())],
            updated: vec![]
        }));
    }

    pub async fn transfer(
        &self,
        user: User,
        selected_resources: Vec<LocalResource>,
        transfer_target: TransferTarget,
        cmd: AppCommandContext
    ) {
        if selected_resources.is_empty() {
            DialogOperation::toast("No valid resources selected".to_string()).into_future(cmd.clone()).await;
            return;
        }

        let transfer_target_id = transfer_target.id();
        let mut transfer_session = match transfer_target {
            TransferTarget::Internet { password, .. } => {
                let session = TransferSession::public(user, password, selected_resources);
                let result = match TransferOperation::create_cloud_session(session).into_future(cmd.clone()).await {
                    Err(err) => {
                        DialogOperation::toast(format!("{err} please try again")).into_future(cmd.clone()).await;
                        return;
                    }
                    Ok(session) => session
                };

                result
            }
            TransferTarget::Nearby(_) => {
                log::info!("Creating nearby session");
                let order_id = PersistentOperation::gen_id().into_future(cmd.clone()).await;
                let result = TransferSession::send(order_id, selected_resources, transfer_target).await;
                log::info!("Created nearby session");

                result
            }
        };

        transfer_session.resources.sort_by(|a, b| a.size.cmp(&b.size));

        cmd.notify_event(AppEvent::Transfer(TransferEvent::UpdateTransferSessions {
            loaded: vec![],
            added: vec![transfer_session.clone()],
            removed: vec![],
            updated: vec![]
        }));

        log::info!(target: "transfer", "Sending resources to peer: {transfer_target_id:?}");

        let mut stream = cmd.stream_from_shell(CoreOperation::Transfer(TransferOperation::SendSession(
            transfer_session.clone()
        )));

        while let Some(output) = stream.next().await {
            match output {
                CoreOperationOutput::Transfer(transfer_output) => match transfer_output {
                    TransferOperationOutput::TransferResourceProgressUpdate(progress) => {
                        if progress.status.is_completed() {
                            log::info!(
                                target: "transfer",
                                "Resource {:?} completed with status {:?}",
                                progress.resource_order_id, progress.status);
                        }

                        transfer_session.update_progress(progress.clone());
                        cmd.notify_event(AppEvent::Transfer(TransferEvent::UpdateResourceTransferProgresses {
                            session_id: transfer_session.order_id,
                            progresses: vec![progress]
                        }));
                    }
                    TransferOperationOutput::TransferCompleted(status) => {
                        if status == TransferSessionStatus::Canceled {
                            transfer_session.cancel();
                        }

                        break;
                    }
                    other => {
                        log::error!(target: "transfer", "Unexpected transfer output: {other:?}");
                        break;
                    }
                },
                CoreOperationOutput::ConnectionError(error) => {
                    log::error!(target: "transfer", "Connection error: {error:?}");
                    transfer_session.force_complete(format!("Connection error: {error:?}"));
                    DialogOperation::toast(format!("{error}")).into_future(cmd.clone()).await;
                    break;
                }
                CoreOperationOutput::DeviceError(error) => {
                    transfer_session.force_complete(format!("Device error: {error:?}"));
                    DialogOperation::toast(format!("{error}")).into_future(cmd.clone()).await;
                    log::error!(target: "transfer", "Device error: {error:?}");
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

        log::info!(target: "transfer", "Transfer session completed");

        // We not remove the public transfer, since user need to see the information
        // after transfer completed.
        if transfer_session.is_success() {
            return;
        }

        cmd.notify_event(AppEvent::Transfer(TransferEvent::UpdateTransferSessions {
            loaded: vec![],
            added: vec![],
            removed: vec![(transfer_session.order_id, transfer_session.transfer_type.clone())],
            updated: vec![]
        }));
    }

    pub async fn received_session_request(&self, remote_session: TransferSessionMessage, peer: Peer, cmd: AppCommandContext) {
        let peer_id = peer.id();
        let generate_file_paths_request = {
            let mut result = HashMap::new();
            for resource in remote_session.resources.iter() {
                result.insert(resource.order_id as u64, resource.name.clone());
            }

            result
        };

        let mut generated_thumbnails_paths =
            TransferSessionPersistentOperation::generate_thumbnail_paths(generate_file_paths_request.keys().copied().collect())
                .into_future(cmd.clone())
                .await;

        let mut generated_saved_paths =
            TransferSessionPersistentOperation::generate_resource_paths(remote_session.order_id, generate_file_paths_request)
                .into_future(cmd.clone())
                .await;

        log::info!(
            target: "transfer",
            "Received session request from peer: {peer_id:?}: {remote_session:?}"
        );

        let mut resources = vec![];
        for resource_request in remote_session.resources {
            let order_id = resource_request.order_id as u64;
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
                order_id: resource_request.order_id as u64
            });
        }

        let mut transfer_session = TransferSession::answer(remote_session.order_id, resources, TransferTarget::Nearby(peer));

        let event = AppEvent::Transfer(TransferEvent::UpdateTransferSessions {
            loaded: vec![],
            added: vec![transfer_session.clone()],
            removed: vec![],
            updated: vec![]
        });

        cmd.notify_event(event);

        let response = CoreOperation::Transfer(TransferOperation::AnswerSessionRequest {
            peer_id: peer_id.to_string(),
            session: Some(transfer_session.clone()),
            session_id: transfer_session.order_id
        });

        let mut stream = cmd.stream_from_shell(response);
        while let Some(transfer_output) = stream.next().await {
            match transfer_output {
                CoreOperationOutput::Transfer(transfer_output) => match transfer_output {
                    TransferOperationOutput::TransferResourceProgressUpdate(progress) => {
                        if progress.status.is_completed() {
                            log::info!(
                                target: "transfer",
                                "Resource {:?} completed with status {:?}",
                                progress.resource_order_id, progress.status
                            );
                        }

                        transfer_session.update_progress(progress.clone());
                        cmd.notify_event(AppEvent::Transfer(TransferEvent::UpdateResourceTransferProgresses {
                            session_id: transfer_session.order_id,
                            progresses: vec![progress]
                        }));
                    }
                    TransferOperationOutput::TransferCompleted(status) => {
                        if status == TransferSessionStatus::Canceled {
                            transfer_session.cancel();
                        }

                        log::info!(target: "transfer", "Transfer session completed with status {:?}", transfer_session.status());
                        break;
                    }
                    TransferOperationOutput::ThumbnailFullFilled {
                        session_id,
                        local_resource_path,
                        resource_id
                    } => {
                        log::info!("Received thumbnail full filled for resource {resource_id}");
                        let request = AppEvent::Transfer(TransferEvent::SessionResourceThumbnailFullFilled {
                            session_id,
                            resource_id,
                            path: local_resource_path
                        });

                        cmd.notify_shell(CoreOperation::Notified(request));
                    }
                    _ => {
                        continue;
                    }
                },
                CoreOperationOutput::ConnectionError(error) => {
                    transfer_session.force_complete(format!("Connection error: {error:?}"));
                    log::error!(target: "transfer", "Connection error: {error:?}");
                    break;
                }
                CoreOperationOutput::DeviceError(error) => {
                    transfer_session.force_complete(format!("Device error: {error:?}"));
                    log::error!(target: "transfer", "Device error: {error:?}");
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

        if matches!(transfer_session.status(), TransferSessionStatus::Canceled) {
            DialogOperation::toast("Transfer session canceled".to_string());

            cmd.notify_event(AppEvent::Transfer(TransferEvent::UpdateTransferSessions {
                loaded: vec![],
                added: vec![],
                removed: vec![(transfer_session.order_id, transfer_session.transfer_type.clone())],
                updated: vec![]
            }));
        } else {
            let progresses = transfer_session.progress.clone();
            cmd.notify_event(AppEvent::Transfer(TransferEvent::UpdateResourceTransferProgresses {
                session_id: transfer_session.order_id,
                progresses
            }));
        }
    }

    pub async fn find_transfer_session(&self, keywords: String, cmd: AppCommandContext) {
        let session_overview = match TransferOperation::find_transfer_session(keywords).into_future(cmd.clone()).await {
            Err(e) => {
                log::error!(target: "transfer", "Failed to find transfer session: {e:?}");
                DialogOperation::toast(format!("{e}")).into_future(cmd.clone()).await;
                return;
            }
            Ok(session_overview) => session_overview
        };

        let Some(session) = session_overview else {
            DialogOperation::message("Not found 🤔".to_owned(), MessageReason::FailedToFindPublicSession)
                .into_future(cmd.clone())
                .await;
            return;
        };

        cmd.notify_event(AppEvent::Transfer(TransferEvent::UpdateTransferSessions {
            loaded: vec![],
            added: vec![session],
            removed: vec![],
            updated: vec![]
        }))
    }

    pub async fn view_public_session(
        &self,
        mut transfer_session: TransferSession,
        entered_password: Option<String>,
        cmd: AppCommandContext
    ) {
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

        let mut stream = cmd.stream_from_shell(request);
        log::info!(target: "transfer", "Subscribing to public transfer session: {transfer_session:?}");
        while let Some(output) = stream.next().await {
            match output {
                CoreOperationOutput::Transfer(transfer) => match transfer {
                    TransferOperationOutput::PublicTransferSessionUpdated((mut resources, mut progresses)) => {
                        for resource in transfer_session.resources.iter_mut() {
                            let Some(updated_index) = resources.iter().position(|r| r.order_id == resource.order_id) else {
                                continue;
                            };

                            *resource = resources.remove(updated_index);
                        }

                        for progress in transfer_session.progress.iter_mut() {
                            let Some(updated_index) =
                                progresses.iter().position(|r| r.resource_order_id == progress.resource_order_id)
                            else {
                                continue;
                            };

                            *progress = progresses.remove(updated_index);
                        }

                        transfer_session.resources.append(&mut resources);
                        transfer_session.progress.append(&mut progresses);

                        log::info!(target: "transfer", "Received public transfer session update: {transfer_session:?}");
                        cmd.notify_event(AppEvent::Transfer(TransferEvent::UpdateTransferSessions {
                            added: vec![],
                            loaded: vec![],
                            removed: vec![],
                            updated: vec![transfer_session.clone()]
                        }));
                    }
                    TransferOperationOutput::SubscribeSessionEnded => {
                        log::info!(target: "transfer", "Public transfer session ended 1");
                        break;
                    }
                    TransferOperationOutput::UnauthenticatedToSubscribeSession => {
                        DialogOperation::message(
                            "Password is not correct".to_owned(),
                            MessageReason::PublicSessionUnauthenticated
                        )
                        .into_future(cmd.clone())
                        .await;
                        return;
                    }
                    _ => return
                },
                CoreOperationOutput::ConnectionError(error) => {
                    DialogOperation::toast(format!("{error}")).into_future(cmd.clone()).await;
                    return;
                }
                _ => return
            };
        }
        log::info!(target: "transfer", "Public transfer session ended");
    }
}
