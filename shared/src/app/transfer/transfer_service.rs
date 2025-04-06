use core_services::local_storage::file_system::File;
use schema::devlog::bitbridge::{ResourceTypeMessage, TransferSessionMessage};

use crate::app::file_system::file::{LocalResource, LocalResourcePath, ResourceType};
use crate::app::modules::transfer::TransferEvent;
use crate::app::operations::database::DatabaseOperation;
use crate::app::operations::local_storage::LocalStorageOperation;
use crate::app::operations::transfer::TransferOperation;
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

    pub async fn transfer(
        &self,
        mut selected_resources: Vec<LocalResource>,
        transfer_targets: Vec<TransferTarget>,
        target_id: String,
        cmd: AppCommandContext
    ) {
        let Some(transfer_target) = transfer_targets.iter().find(|it| it.id() == target_id) else {
            return;
        };

        for resource in selected_resources.iter_mut() {
            resource.path = LocalResourcePath::LocalPath(
                LocalStorageOperation::get_absolute_path(resource.path.clone()).into_future(cmd.clone()).await
            );
            resource.thumbnail_path = match resource.thumbnail_path.clone() {
                Some(path) => Some(LocalResourcePath::LocalPath(
                    LocalStorageOperation::get_absolute_path(path).into_future(cmd.clone()).await
                )),
                _ => None
            };
        }

        let order_id = DatabaseOperation::gen_id().into_future(cmd.clone()).await;
        let transfer_session = TransferSession {
            order_id,
            resources: selected_resources,
            progress: vec![],
            transfer_type: TransferType::Send,
            target: transfer_target.clone()
        };

        TransferOperation::send_session(transfer_session.clone()).into_future(cmd.clone()).await;
        cmd.send_event(AppEvent::Transfer(TransferEvent::UpdateTransferSessions {
            new: vec![transfer_session],
            removed: vec![]
        }));
    }

    pub async fn received_session_request(&self, request: TransferSessionMessage, peer: Peer, cmd: AppCommandContext) {
        let mut resources = vec![];
        let workdir = LocalStorageOperation::get_work_dir_path_cmd().into_future(cmd.clone()).await;
        for resource_request in request.resources {
            let thumbnail_path = format!("{}/thumbnails/{}.png", workdir, resource_request.order_id);
            let thumbnail_file_path = File::new(resource_request.thumbnail_png, thumbnail_path.clone())
                .await
                .ok()
                .map(|_it| LocalResourcePath::LocalPath(thumbnail_path.clone()));

            resources.push(LocalResource {
                path: LocalResourcePath::LocalPath(format!(
                    "{}/sessions/{}/{}/{}",
                    workdir, request.order_id, resource_request.order_id, resource_request.name
                )),
                thumbnail_path: thumbnail_file_path,
                r#type: ResourceType::from(
                    ResourceTypeMessage::try_from(resource_request.r#type).unwrap_or(ResourceTypeMessage::Other)
                ),
                name: resource_request.name,
                size: resource_request.size as u64,
                order_id: resource_request.order_id as u64
            });
        }

        let transfer_session = TransferSession {
            order_id: request.order_id,
            progress: resources.iter().map(|it| TransferProgress::new(it.order_id)).collect(),
            resources,
            transfer_type: TransferType::Receive,
            target: TransferTarget::Nearby(peer)
        };

        cmd.send_event(AppEvent::Transfer(TransferEvent::UpdateTransferSessions {
            new: vec![transfer_session],
            removed: vec![]
        }));
    }
}
