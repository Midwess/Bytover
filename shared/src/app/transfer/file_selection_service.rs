use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::app::modules::transfer::TransferEvent;
use crate::app::operations::device::DeviceOperation;
use crate::app::operations::persistent::LocalResourcePersistentOperation;
use crate::app::operations::CoreOperation;
use crate::app::{AppCommandContext, AppEvent};
use crate::entities::file_system::file::{LocalResourcePath, ResourceType};

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub struct ResourceSelection {
    pub path: LocalResourcePath,
    // This is optional, if it is None, we will detect by Rust code to see if it should be a Folder or a File
    pub r#type: Option<ResourceType>
}

pub struct ResourceTransferSelectionService {}

impl ResourceTransferSelectionService {
    pub fn instance() -> &'static ResourceTransferSelectionService {
        static INSTANCE: OnceLock<ResourceTransferSelectionService> = OnceLock::new();
        INSTANCE.get_or_init(|| ResourceTransferSelectionService {})
    }

    pub async fn load_resources(&self, ctx: AppCommandContext) {
        let resources = LocalResourcePersistentOperation::find_all().into_future(ctx.clone()).await;

        ctx.send_event(AppEvent::Transfer(TransferEvent::UpdateResourcesModel {
            loaded: resources.clone(),
            added: vec![],
            removed: vec![],
            updated: vec![]
        }));

        ctx.request_from_shell(CoreOperation::Render).await;
    }

    pub async fn add_resources(&self, ctx: AppCommandContext, mut selections: Vec<ResourceSelection>) {
        while let Some(selection) = selections.pop() {
            let existing_resource = LocalResourcePersistentOperation::find(selection.path.clone()).into_future(ctx.clone()).await;
            if existing_resource.is_some() {
                log::info!("File already exists: {:?}", selection.path);
                continue;
            }

            let Some(mut local_resource) = LocalResourcePersistentOperation::load_from_disk(selection.path.clone())
                .into_future(ctx.clone())
                .await
            else {
                log::error!("File not exists: {:?}", selection.path);
                continue;
            };

            local_resource.path = selection.path.clone();
            local_resource.r#type = match selection.r#type.clone() {
                Some(r#type) => r#type,
                None => {
                    LocalResourcePersistentOperation::get_resource_type(selection.path.clone())
                        .into_future(ctx.clone())
                        .await
                }
            };

            let mut new_resources = LocalResourcePersistentOperation::add(vec![local_resource.clone()]).into_future(ctx.clone()).await;
            if new_resources.is_empty() {
                log::info!("File already exists: {:?}", selection.path);
                continue;
            }

            let mut new_resource = new_resources.pop().unwrap();

            match DeviceOperation::load_thumbnail_png(selection.path.clone()).into_future(ctx.clone()).await {
                (Some(thumbnail_png), _) => {
                    let thumbnail = LocalResourcePersistentOperation::add_thumbnail(thumbnail_png, local_resource.order_id)
                        .into_future(ctx.clone())
                        .await;

                    new_resource.thumbnail_path = Some(thumbnail);
                }
                (_, Some(thumbnail_path)) => {
                    new_resource.thumbnail_path = Some(thumbnail_path);
                }
                _ => {}
            };

            ctx.send_event(AppEvent::Transfer(TransferEvent::UpdateResourcesModel {
                loaded: vec![],
                added: vec![new_resource],
                removed: vec![],
                updated: vec![]
            }));
        }
    }

    pub async fn remove_resource(&self, ctx: AppCommandContext, id: u64) {
        let removed_resource = LocalResourcePersistentOperation::remove(id).into_future(ctx.clone()).await;
        if let Some(removed_resource) = removed_resource {
            ctx.send_event(AppEvent::Transfer(TransferEvent::UpdateResourcesModel {
                loaded: vec![],
                added: vec![],
                removed: vec![removed_resource],
                updated: vec![]
            }));
        }
    }
}
