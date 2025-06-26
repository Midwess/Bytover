use std::sync::OnceLock;

use serde::{Deserialize, Serialize};
use uniffi::Record;

use crate::app::file_system::file::{LocalResourcePath, ResourceType};
use crate::app::modules::transfer::TransferEvent;
use crate::app::operations::database::{DatabaseOperation, LocalResourceDatabaseOperation};
use crate::app::operations::local_storage::LocalStorageOperation;
use crate::app::operations::CoreOperation;
use crate::app::{AppCommandContext, AppEvent};

#[derive(Debug, PartialEq, Eq, Record, Serialize, Deserialize, Clone)]
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
        let mut resources = LocalResourceDatabaseOperation::find_all().into_future(ctx.clone()).await;

        ctx.send_event(AppEvent::Transfer(TransferEvent::UpdateResourcesModel {
            loaded: resources.clone(),
            new: vec![],
            removed: vec![],
            updated: vec![]
        }));

        ctx.request_from_shell(CoreOperation::Render).await;

        let mut updated_resources = vec![];
        for resource in resources.iter_mut() {
            if resource.validate(ctx.clone()).await {
                updated_resources.push(resource.clone());
            }
        }

        if !updated_resources.is_empty() {
            ctx.send_event(AppEvent::Transfer(TransferEvent::UpdateResourcesModel {
                loaded: vec![],
                new: vec![],
                removed: vec![],
                updated: updated_resources
            }));

            ctx.request_from_shell(CoreOperation::Render).await;
        }
    }

    pub async fn add_resources(&self, ctx: AppCommandContext, mut selections: Vec<ResourceSelection>) {
        while let Some(selection) = selections.pop() {
            let existing_resource = LocalResourceDatabaseOperation::find(selection.path.clone()).into_future(ctx.clone()).await;
            if existing_resource.is_some() {
                continue;
            }

            let order_id = DatabaseOperation::gen_id().into_future(ctx.clone()).await;

            let Some(mut local_resource) = LocalStorageOperation::get(selection.path.clone()).into_future(ctx.clone()).await else {
                log::error!(target: "transfer", "Failed to get resource: {:?}", selection.path);
                continue;
            };

            local_resource.path = selection.path.clone();

            local_resource.r#type = match selection.r#type.clone() {
                Some(r#type) => r#type,
                None => match LocalStorageOperation::get_resource_type(selection.path.clone()).into_future(ctx.clone()).await {
                    Some(resource_type) => resource_type,
                    None => {
                        log::error!(target: "transfer", "Failed to get resource type, the file might no longer exist");
                        continue;
                    }
                }
            };

            if let Some(thumbnail_png) = LocalStorageOperation::load_file_thumbnail_png(selection.path.clone())
                .into_future(ctx.clone())
                .await
            {
                let thumbnail = LocalStorageOperation::new_thumbnail(thumbnail_png, order_id).into_future(ctx.clone()).await;

                local_resource.thumbnail_path = Some(thumbnail.path);
            }

            let new_resources = LocalResourceDatabaseOperation::add(vec![local_resource]).into_future(ctx.clone()).await;
            ctx.send_event(AppEvent::Transfer(TransferEvent::UpdateResourcesModel {
                loaded: vec![],
                new: new_resources,
                removed: vec![],
                updated: vec![]
            }));
        }

        ctx.send_event(AppEvent::Transfer(TransferEvent::UpdateResourcesModel {
            loaded: vec![],
            new: vec![],
            removed: vec![],
            updated: vec![]
        }));
    }

    pub async fn remove_resource(&self, ctx: AppCommandContext, id: u64) {
        let removed_resource = LocalResourceDatabaseOperation::remove(id).into_future(ctx.clone()).await;
        if let Some(removed_resource) = removed_resource {
            ctx.send_event(AppEvent::Transfer(TransferEvent::UpdateResourcesModel {
                loaded: vec![],
                new: vec![],
                removed: vec![removed_resource],
                updated: vec![]
            }));
        }
    }
}
