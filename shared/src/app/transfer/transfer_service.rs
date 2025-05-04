use futures_util::StreamExt;
use schema::devlog::bitbridge::peer_message_body::Response;
use schema::devlog::bitbridge::{ResourceTypeMessage, TransferResponseMessage, TransferSessionMessage};

use crate::app::file_system::file::{LocalResource, LocalResourcePath, ResourceType};
use crate::app::modules::transfer::TransferEvent;
use crate::app::operations::dialog::{AlertDialog, DialogOperation};
use crate::app::operations::local_storage::LocalStorageOperation;
use crate::app::operations::transfer::{TransferOperation, TransferOperationOutput};
use crate::app::operations::{CoreOperation, CoreOperationOutput};
use crate::app::transfer::session::TransferSessionStatus;
use crate::app::{AppCommandContext, AppEvent};
use crate::entities::peer::Peer;

use super::session::TransferSession;
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

    pub async fn cancel_transfer(&self, transfer_session: TransferSession, cmd: AppCommandContext) -> bool {
        let status = transfer_session.status();
        if matches!(status, TransferSessionStatus::Failed(_) | TransferSessionStatus::Success) {
            return false;
        }

        // If not canceled, ask for confirmation
        if transfer_session.status() != TransferSessionStatus::Canceled {
            let confirmation = DialogOperation::alert(AlertDialog::confirmation(
                "Cancel the transfer ?".to_string(),
                "Yes".to_string(),
                Some("No".to_string())
            ))
            .into_future(cmd.clone())
            .await;

            if !confirmation {
                return false;
            }
        }

        log::info!(target: "transfer", "Cancelling transfer: {:?}", transfer_session.order_id);

        cmd.request_from_shell(CoreOperation::Transfer(TransferOperation::CancelSession(
            transfer_session.peer_id().unwrap(),
            transfer_session.order_id
        )))
        .await;

        cmd.send_event(AppEvent::Transfer(TransferEvent::UpdateTransferSessions {
            new: vec![],
            removed: vec![transfer_session.clone()],
            updated: vec![]
        }));

        cmd.notify_shell(CoreOperation::Render);

        true
    }

    pub async fn transfer(&self, mut selected_resources: Vec<LocalResource>, transfer_target: TransferTarget, cmd: AppCommandContext) {
        let mut update_resources = vec![];
        for selected_resource in selected_resources.iter_mut() {
            if selected_resource.validate(cmd.clone()).await {
                update_resources.push(selected_resource.clone());
            }
        }

        if !update_resources.is_empty() {
            cmd.send_event(AppEvent::Transfer(TransferEvent::UpdateResourcesModel {
                new: vec![],
                removed: vec![],
                updated: update_resources
            }));

            cmd.notify_shell(CoreOperation::Render);
        }

        let selected_resources = selected_resources.into_iter().filter(|it| it.is_valid).collect::<Vec<_>>();
        if selected_resources.is_empty() {
            DialogOperation::toast("No valid resources selected".to_string()).into_future(cmd.clone()).await;
            return;
        }

        let transfer_target_id = transfer_target.id();
        let mut transfer_session = TransferSession::send(selected_resources, transfer_target).await;

        cmd.send_event(AppEvent::Transfer(TransferEvent::UpdateTransferSessions {
            new: vec![transfer_session.clone()],
            updated: vec![],
            removed: vec![]
        }));

        cmd.notify_shell(CoreOperation::Render);

        for resource in transfer_session.resources.iter_mut() {
            resource.path = match LocalStorageOperation::get_absolute_path(resource.path.clone()).into_future(cmd.clone()).await {
                Some(path) => LocalResourcePath::LocalPath(path),
                None => {
                    resource.is_valid = false;
                    cmd.send_event(AppEvent::Transfer(TransferEvent::UpdateResourcesModel {
                        new: vec![],
                        removed: vec![],
                        updated: vec![resource.clone()]
                    }));

                    cmd.notify_shell(CoreOperation::Render);
                    continue;
                }
            };

            resource.thumbnail_path = match resource.thumbnail_path.clone() {
                Some(path) => LocalStorageOperation::get_absolute_path(path)
                    .into_future(cmd.clone())
                    .await
                    .map(LocalResourcePath::LocalPath),
                _ => None
            };
        }

        transfer_session.resources.sort_by(|a, b| a.size.cmp(&b.size));

        cmd.send_event(AppEvent::Transfer(TransferEvent::UpdateTransferSessions {
            new: vec![],
            updated: vec![transfer_session.clone()],
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

                        transfer_session.update_progress(progress);
                        cmd.send_event(AppEvent::Transfer(TransferEvent::UpdateTransferSessions {
                            new: vec![],
                            removed: vec![],
                            updated: vec![transfer_session.clone()]
                        }));

                        cmd.notify_shell(CoreOperation::Render);
                    }
                    other => {
                        log::error!(target: "transfer", "Unexpected transfer output: {:?}", other);
                        break;
                    }
                },
                CoreOperationOutput::ConnectionError(error) => {
                    transfer_session.force_complete(format!("Connection error: {:?}", error));
                    log::error!(target: "transfer", "Connection error: {:?}", error);
                    break;
                }
                CoreOperationOutput::DeviceError(error) => {
                    transfer_session.force_complete(format!("Device error: {:?}", error));
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

        cmd.send_event(AppEvent::Transfer(TransferEvent::UpdateTransferSessions {
            new: vec![],
            removed: vec![transfer_session.clone()],
            updated: vec![]
        }));

        cmd.notify_shell(CoreOperation::Render);
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
        for resource_request in remote_session.resources {
            let thumbnail_path = match resource_request.thumbnail_png {
                Some(thumbnail_png) => {
                    let thumbnail_path = format!("{}/thumbnails/{}.png", workdir, resource_request.order_id);
                    Some(
                        LocalStorageOperation::new_file(thumbnail_png, thumbnail_path.clone())
                            .into_future(cmd.clone())
                            .await
                            .path
                    )
                }
                None => None
            };

            resources.push(LocalResource {
                is_valid: true,
                path: LocalResourcePath::LocalPath(format!(
                    "{}/sessions/{}/{}/{}",
                    workdir, remote_session.order_id, resource_request.order_id, resource_request.name
                )),
                thumbnail_path,
                r#type: ResourceType::from(
                    ResourceTypeMessage::try_from(resource_request.r#type).unwrap_or(ResourceTypeMessage::Other)
                ),
                name: resource_request.name,
                size: resource_request.size as u64,
                order_id: resource_request.order_id as u64
            });
        }

        let mut transfer_session = TransferSession::answer(remote_session.order_id, resources, TransferTarget::Nearby(peer)).await;

        cmd.send_event(AppEvent::Transfer(TransferEvent::UpdateTransferSessions {
            new: vec![transfer_session.clone()],
            removed: vec![],
            updated: vec![]
        }));

        cmd.notify_shell(CoreOperation::Render);

        let response = Response::TransferResponse(TransferResponseMessage {});
        let response = CoreOperation::Transfer(TransferOperation::AnswerSessionRequest(
            peer_id,
            transfer_session.clone(),
            request_id,
            response
        ));

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

                        transfer_session.update_progress(progress);
                        cmd.send_event(AppEvent::Transfer(TransferEvent::UpdateTransferSessions {
                            new: vec![],
                            removed: vec![],
                            updated: vec![transfer_session.clone()]
                        }));

                        cmd.notify_shell(CoreOperation::Render);
                    }
                    _ => {
                        continue;
                    }
                },
                CoreOperationOutput::ConnectionError(error) => {
                    transfer_session.force_complete(format!("Connection error: {:?}", error));
                    log::error!(target: "transfer", "Connection error: {:?}", error);
                    break;
                }
                CoreOperationOutput::DeviceError(error) => {
                    transfer_session.force_complete(format!("Device error: {:?}", error));
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
    }
}
