use devlog_sdk::distributed_id::gen_id;
use serde::{Deserialize, Serialize};
use uniffi::Record;

use crate::app::file_system::file::{LocalResource, LocalResourcePath, ResourceType};
use crate::app::modules::transfer::TransferEvent;
use crate::app::operations::database::LocalResourceDatabaseOperation;
use crate::app::operations::local_storage::LocalStorageOperation;
use crate::app::operations::CoreOperation;
use crate::app::{AppCommandContext, AppEvent};

#[derive(Debug, PartialEq, Eq, Record, Serialize, Deserialize, Clone)]
pub struct ResourceSelection {
    pub path: LocalResourcePath,
    pub r#type: ResourceType
}

pub struct ResourceTransferSelectionService {}

impl ResourceTransferSelectionService {
    pub async fn load_resources(&self, ctx: AppCommandContext) {
        let resources = LocalResourceDatabaseOperation::find_all().into_future(ctx.clone()).await;
        ctx.send_event(AppEvent::Transfer(TransferEvent::UpdateResourcesModel {
            new: resources,
            removed: vec![]
        }));

        ctx.request_from_shell(CoreOperation::Render).await;
    }

    pub async fn add_resources(&self, ctx: AppCommandContext, selections: Vec<ResourceSelection>) {
        let mut local_resources = vec![];
        let workdir = LocalStorageOperation::get_work_dir_path_cmd().into_future(ctx.clone()).await;
        for selection in selections {
            let existing_resource = LocalResourceDatabaseOperation::find(selection.path.clone()).into_future(ctx.clone()).await;
            if existing_resource.is_some() {
                continue;
            }

            let order_id = gen_id().await;
            let local_resource = match selection.path {
                LocalResourcePath::LocalPath(path) => {
                    let resource = LocalStorageOperation::get(path).into_future(ctx.clone()).await;
                    if resource.is_none() {
                        panic!("Resource not found")
                    }

                    resource.unwrap()
                }
                LocalResourcePath::PlatformIdentifier(identifier) => {
                    let file_size = LocalStorageOperation::load_file_size_from_platform_identifier(identifier.clone())
                        .into_future(ctx.clone())
                        .await;

                    let file_name = LocalStorageOperation::load_file_name_from_platform_identifier(identifier.clone())
                        .into_future(ctx.clone())
                        .await;

                    let mut thumbnail_path = None;
                    if let Some(thumbnail_png) =
                        LocalStorageOperation::load_file_thumbnail_png_from_platform_identifier(identifier.clone())
                            .into_future(ctx.clone())
                            .await
                    {
                        let path = format!("thumbnails/{}.png", order_id);
                        let absolute_path = format!("{}/{}", workdir, path);
                        let _ = LocalStorageOperation::new_file(thumbnail_png, absolute_path).into_future(ctx.clone()).await;

                        thumbnail_path = Some(LocalResourcePath::LocalPath(path));
                    }

                    LocalResource {
                        order_id,
                        name: file_name,
                        size: file_size,
                        path: LocalResourcePath::PlatformIdentifier(identifier),
                        thumbnail_path,
                        r#type: selection.r#type
                    }
                }
            };

            local_resources.push(local_resource);
        }

        let new_resources = LocalResourceDatabaseOperation::add(local_resources).into_future(ctx.clone()).await;
        ctx.send_event(AppEvent::Transfer(TransferEvent::UpdateResourcesModel {
            new: new_resources,
            removed: vec![]
        }));

        ctx.request_from_shell(CoreOperation::Render).await;
    }

    pub async fn remove_resource(&self, ctx: AppCommandContext, id: u64) {
        log::info!(target: "transfer", "Remove resource {:?}", id);
        let removed_resource = LocalResourceDatabaseOperation::remove(id).into_future(ctx.clone()).await;
        if let Some(removed_resource) = removed_resource {
            ctx.send_event(AppEvent::Transfer(TransferEvent::UpdateResourcesModel {
                new: vec![],
                removed: vec![removed_resource]
            }));
        }

        ctx.request_from_shell(CoreOperation::Render).await;
    }
}
