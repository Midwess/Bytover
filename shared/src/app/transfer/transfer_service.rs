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
        let transfer_session = TransferSession {
            order_id,
            progress: selected_resources.iter().map(|it| TransferProgress::new(it.order_id)).collect(),
            resources: selected_resources,
            transfer_type: TransferType::Send,
            target: transfer_target.clone()
        };

        TransferOperation::send_session(transfer_session.clone()).into_future(cmd.clone()).await;

        let mut sorted_resources: Vec<_> = transfer_session.resources.iter().collect();
        sorted_resources.sort_by(|a, b| a.size.cmp(&b.size));

        let resources = sorted_resources;

        for resource in resources {
            let peer_id = transfer_target.id().parse::<u128>().unwrap_or(0);
            TransferOperation::send_resource(peer_id, order_id, resource.clone()).into_future(cmd.clone()).await;
        }

        cmd.send_event(AppEvent::Transfer(TransferEvent::UpdateTransferSessions {
            new: vec![transfer_session],
            removed: vec![]
        }));
    }

    pub async fn received_session_request(&self, request: TransferSessionMessage, peer: Peer, cmd: AppCommandContext) {
        let mut resources = vec![];
        let workdir = LocalStorageOperation::get_work_dir_path_cmd().into_future(cmd.clone()).await;
        for resource_request in request.resources {
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
                    workdir, request.order_id, resource_request.order_id, resource_request.name
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
