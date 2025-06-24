use futures_util::StreamExt;
use schema::devlog::bitbridge::peer_message_body::Response;
use schema::devlog::bitbridge::{ResourceTypeMessage, TransferResponseMessage, TransferSessionMessage};

use crate::app::core_utils::CoreCommandContextUtils;
use crate::app::file_system::file::{LocalResource, LocalResourcePath, ResourceType};
use crate::app::modules::transfer::TransferEvent;
use crate::app::operations::database::TransferSessionOperation;
use crate::app::operations::dialog::DialogOperation;
use crate::app::operations::local_storage::LocalStorageOperation;
use crate::app::operations::transfer::{TransferOperation, TransferOperationOutput};
use crate::app::operations::{CoreOperation, CoreOperationOutput};
use crate::app::transfer::session::TransferSessionStatus;
use crate::app::{AppCommandContext, AppEvent};
use crate::entities::peer::Peer;
use crate::persistence::transfer_session::TransferSessionId;

use super::session::{TransferSession, TransferType};
use super::target::TransferTarget;

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

    pub async fn load_transfer_sessions(&self, cmd: AppCommandContext) {
        log::info!(target: "transfer", "Loading transfer sessions");
        let receive_session_id = TransferSessionId {
            transfer_type: Some(TransferType::Receive),
            ..Default::default()
        };

        let receive_sessions = TransferSessionOperation::get_all(receive_session_id).into_future(cmd.clone()).await;

        let event = AppEvent::Transfer(TransferEvent::UpdateTransferSessions {
            loaded: receive_sessions,
            new: vec![],
            removed: vec![]
        });

        cmd.notify_event(event);
    }

    pub async fn delete_session(&self, transfer_session: TransferSession, cmd: AppCommandContext) {
        if !transfer_session.is_completed() {
            log::info!(target: "transfer", "Cancelling transfer: {:?}", transfer_session.order_id);

            if let Err(error) = TransferOperation::cancel_session(transfer_session.peer_id(), transfer_session.order_id)
                .into_future(cmd.clone())
                .await
            {
                log::error!(target: "transfer", "Failed to cancel transfer: {:?}", error);
            }
        }

        let workdir = LocalStorageOperation::get_work_dir_path_cmd().into_future(cmd.clone()).await;
        let path = workdir.session_folder(transfer_session.order_id);
        let _ = LocalStorageOperation::delete(LocalResourcePath::AbsolutePath(path)).into_future(cmd.clone()).await;

        cmd.notify_event(AppEvent::Transfer(TransferEvent::UpdateTransferSessions {
            loaded: vec![],
            new: vec![],
            removed: vec![transfer_session.order_id]
        }));
    }

    pub async fn transfer(&self, mut selected_resources: Vec<LocalResource>, transfer_target: TransferTarget, cmd: AppCommandContext) {
        let mut update_resources = vec![];
        for selected_resource in selected_resources.iter_mut() {
            if selected_resource.validate(cmd.clone()).await {
                update_resources.push(selected_resource.clone());
            }
        }

        if !update_resources.is_empty() {
            cmd.notify_event(AppEvent::Transfer(TransferEvent::UpdateResourcesModel {
                loaded: vec![],
                new: vec![],
                removed: vec![],
                updated: update_resources
            }));
        }

        let selected_resources = selected_resources.into_iter().filter(|it| it.is_valid).collect::<Vec<_>>();
        if selected_resources.is_empty() {
            DialogOperation::toast("No valid resources selected".to_string()).into_future(cmd.clone()).await;
            return;
        }

        let transfer_target_id = transfer_target.id();
        let mut transfer_session = match transfer_target {
            TransferTarget::Internet { password, .. } => {
                let session = TransferSession::public(password, selected_resources);
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
                let result = TransferSession::send(selected_resources, transfer_target).await;
                log::info!("Created nearby session");

                result
            }
        };

        for resource in transfer_session.resources.iter_mut() {
            resource.is_valid = LocalStorageOperation::is_file_exists(resource.path.clone()).into_future(cmd.clone()).await;
            if !resource.is_valid {
                cmd.notify_event(AppEvent::Transfer(TransferEvent::UpdateResourcesModel {
                    loaded: vec![],
                    new: vec![],
                    removed: vec![],
                    updated: vec![resource.clone()]
                }));

                continue;
            }

            resource.path = LocalResourcePath::AbsolutePath(
                LocalStorageOperation::get_absolute_path(resource.path.clone()).into_future(cmd.clone()).await
            );

            resource.thumbnail_path = match &resource.thumbnail_path {
                Some(path) => Some(LocalResourcePath::AbsolutePath(
                    LocalStorageOperation::get_absolute_path(path.clone()).into_future(cmd.clone()).await
                )),
                None => None
            };

            if resource.r#type == ResourceType::Folder {
                resource.name = format!("{}.tar", resource.name);
            }
        }

        transfer_session.resources.sort_by(|a, b| a.size.cmp(&b.size));

        cmd.notify_event(AppEvent::Transfer(TransferEvent::UpdateTransferSessions {
            loaded: vec![],
            new: vec![transfer_session.clone()],
            removed: vec![]
        }));

        log::info!(target: "transfer", "Sending resources to peer: {:?}", transfer_target_id);

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
                        log::error!(target: "transfer", "Unexpected transfer output: {:?}", other);
                        break;
                    }
                },
                CoreOperationOutput::ConnectionError(error) => {
                    transfer_session.force_complete(format!("Connection error: {error:?}"));
                    log::error!(target: "transfer", "Connection error: {:?}", error);
                    break;
                }
                CoreOperationOutput::DeviceError(error) => {
                    transfer_session.force_complete(format!("Device error: {error:?}"));
                    log::error!(target: "transfer", "Device error: {:?}", error);
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
        if transfer_session.target.is_public() {
            return;
        }

        cmd.notify_event(AppEvent::Transfer(TransferEvent::UpdateTransferSessions {
            loaded: vec![],
            new: vec![],
            removed: vec![transfer_session.order_id]
        }));
    }

    pub async fn received_session_request(
        &self,
        (request_id, remote_session): (String, TransferSessionMessage),
        peer: Peer,
        cmd: AppCommandContext
    ) {
        let peer_id = peer.id();
        let mut resources = vec![];
        let workdir = LocalStorageOperation::get_work_dir_path_cmd().into_future(cmd.clone()).await;
        let thumbnail_dir = workdir.thumbnails("".to_string());
        for resource_request in remote_session.resources {
            resources.push(LocalResource {
                is_valid: true,
                path: LocalResourcePath::AbsolutePath(workdir.resources(remote_session.order_id, resource_request.name.clone())),
                thumbnail_path: None,
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
            new: vec![transfer_session.clone()],
            removed: vec![]
        });

        cmd.notify_event(event);

        let response = Response::TransferResponse(TransferResponseMessage {});
        let response = CoreOperation::Transfer(TransferOperation::AnswerSessionRequest {
            thumbnail_dir,
            peer_id,
            session: transfer_session.clone(),
            peer_request_id: request_id,
            response
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
                    _ => {
                        continue;
                    }
                },
                CoreOperationOutput::ConnectionError(error) => {
                    transfer_session.force_complete(format!("Connection error: {error:?}"));
                    log::error!(target: "transfer", "Connection error: {:?}", error);
                    break;
                }
                CoreOperationOutput::DeviceError(error) => {
                    transfer_session.force_complete(format!("Device error: {error:?}"));
                    log::error!(target: "transfer", "Device error: {:?}", error);
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
                new: vec![],
                removed: vec![transfer_session.order_id]
            }));
        } else {
            let progresses = transfer_session.progress.clone();
            cmd.notify_event(AppEvent::Transfer(TransferEvent::UpdateResourceTransferProgresses {
                session_id: transfer_session.order_id,
                progresses
            }));
        }
    }
}
