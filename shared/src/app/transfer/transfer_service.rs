use futures_util::StreamExt;
use schema::devlog::bitbridge::peer_message_body::Response;
use schema::devlog::bitbridge::{ResourceTypeMessage, TransferResponseMessage, TransferSessionMessage};

use crate::app::file_system::file::{LocalResource, LocalResourcePath, ResourceType};
use crate::app::modules::transfer::TransferEvent;
use crate::app::operations::database::DatabaseOperation;
use crate::app::operations::local_storage::LocalStorageOperation;
use crate::app::operations::transfer::{TransferOperation, TransferOperationOutput};
use crate::app::operations::{CoreOperation, CoreOperationOutput};
use crate::app::{AppCommandContext, AppEvent};
use crate::entities::peer::Peer;

use super::session::{TransferProgress, TransferSession, TransferType};
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

    pub async fn transfer(&self, mut selected_resources: Vec<LocalResource>, transfer_target: TransferTarget, cmd: AppCommandContext) {
        if selected_resources.is_empty() {
            return;
        }

        for resource in selected_resources.iter_mut() {
            resource.path = match LocalStorageOperation::get_absolute_path(resource.path.clone()).into_future(cmd.clone()).await {
                Some(path) => LocalResourcePath::LocalPath(path),
                None => continue
            };

            resource.thumbnail_path = match resource.thumbnail_path.clone() {
                Some(path) => LocalStorageOperation::get_absolute_path(path)
                    .into_future(cmd.clone())
                    .await
                    .map(LocalResourcePath::LocalPath),
                _ => None
            };
        }

        let order_id = DatabaseOperation::gen_id().into_future(cmd.clone()).await;

        let mut transfer_session = TransferSession {
            order_id,
            progress: selected_resources.iter().map(|it| TransferProgress::new(it.order_id)).collect(),
            resources: selected_resources,
            transfer_type: TransferType::Send,
            target: transfer_target.clone()
        };

        transfer_session.resources.sort_by(|a, b| a.size.cmp(&b.size));

        log::info!(target: "transfer", "Sending resources to peer: {:?}", transfer_target.id());

        cmd.send_event(AppEvent::Transfer(TransferEvent::UpdateTransferSessions {
            new: vec![transfer_session.clone()],
            updated: vec![],
            removed: vec![]
        }));

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

        let mut transfer_session = TransferSession {
            order_id: remote_session.order_id,
            progress: resources.iter().map(|it| TransferProgress::new(it.order_id)).collect(),
            resources: resources.clone(),
            transfer_type: TransferType::Receive,
            target: TransferTarget::Nearby(peer)
        };

        cmd.send_event(AppEvent::Transfer(TransferEvent::UpdateTransferSessions {
            new: vec![transfer_session.clone()],
            removed: vec![],
            updated: vec![]
        }));

        let response = Response::TransferResponse(TransferResponseMessage {});
        let response = CoreOperation::Transfer(TransferOperation::AnswerSessionRequest(
            peer_id,
            resources,
            transfer_session.order_id,
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
