use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use surrealdb::Uuid;
use uniffi::{Enum, Record};

use crate::app::modules::transfer::TransferEvent;
use crate::app::operations::local_storage::LocalStorageOperation;
use crate::app::operations::CoreOperation;
use crate::app::{AppCommandContext, AppEvent};
use crate::entities::file::{LocalResource, LocalResourcePath, ResourceType};

#[derive(Debug, PartialEq, Eq, Record, Serialize, Deserialize, Clone)]
pub struct ResourceSelection {
    data: ResourceSelectionData,
    r#type: ResourceType,
    name: String
}

#[derive(Debug, PartialEq, Eq, Enum, Serialize, Deserialize, Clone)]
pub enum ResourceSelectionData {
    Bytes(Vec<u8>),
    LocalPath(String),
    // The only way to load resource is through the platform
    // Eg: iOS has itemIdentifier to identify the photo
    PlatformIdentifier(String)
}

impl ResourceSelection {
    pub fn path(&self) -> &String {
        match &self.data {
            ResourceSelectionData::LocalPath(path) => path,
            ResourceSelectionData::PlatformIdentifier(identifier) => identifier,
            ResourceSelectionData::Bytes(_) => panic!("Resource selection data is not a path")
        }
    }
}

impl From<LocalResourcePath> for ResourceSelectionData {
    fn from(path: LocalResourcePath) -> Self {
        match path {
            LocalResourcePath::LocalPath(path) => ResourceSelectionData::LocalPath(path),
            LocalResourcePath::PlatformIdentifier(identifier) => ResourceSelectionData::PlatformIdentifier(identifier)
        }
    }
}

impl From<&LocalResource> for ResourceSelection {
    fn from(resource: &LocalResource) -> Self {
        ResourceSelection {
            data: ResourceSelectionData::from(resource.path.clone()),
            r#type: resource.r#type.clone(),
            name: resource.name.clone()
        }
    }
}

pub struct ResourceTransferSelectionService {}

impl ResourceTransferSelectionService {
    pub async fn add_resources(
        &self,
        ctx: AppCommandContext,
        selections_from_core: Vec<LocalResource>,
        selections_from_shell: Vec<ResourceSelection>
    ) {
        let mut local_resources = selections_from_core
            .into_iter()
            .filter(|it| {
                !selections_from_shell.iter().any(|selection| match &it.path {
                    LocalResourcePath::LocalPath(path) => path.eq(selection.path()),
                    LocalResourcePath::PlatformIdentifier(identifier) => identifier.eq(selection.path())
                })
            })
            .collect::<Vec<_>>();

        for selection in selections_from_shell {
            let local_resource = match selection.data {
                ResourceSelectionData::Bytes(bytes) => {
                    let new_name = Uuid::new_v4().to_string();
                    let extension = PathBuf::from(selection.name).extension().map(|it| it.to_string_lossy().to_string()).unwrap_or_else(|| "".to_string());
                    let new_path = format!("transfer/{}.{}", new_name, extension);
                    let absolute_path = LocalStorageOperation::get_absolute_path(new_path, ctx.clone()).await;
                    LocalStorageOperation::new_file(bytes, absolute_path).into_future(ctx.clone()).await
                }
                ResourceSelectionData::LocalPath(path) => {
                    let resource = LocalStorageOperation::get(path).into_future(ctx.clone()).await;
                    if resource.is_none() {
                        panic!("Resource not found")
                    }

                    resource.unwrap()
                }
                ResourceSelectionData::PlatformIdentifier(identifier) => {
                    let file_size = LocalStorageOperation::load_file_size_from_platform_identifier(identifier.clone())
                        .into_future(ctx.clone())
                        .await;
                    let file_name = LocalStorageOperation::load_file_name_from_platform_identifier(identifier.clone())
                        .into_future(ctx.clone())
                        .await;

                    let mut thumbnail_path = None;
                    if let Some(thumbnail_png) = LocalStorageOperation::load_file_thumbnail_png_from_platform_identifier(identifier.clone()).into_future(ctx.clone()).await {
                        let path = format!("thumbnails/{}.png", file_name);
                        let absolute_path = LocalStorageOperation::get_absolute_path(path.clone(), ctx.clone()).await;
                        let _ = LocalStorageOperation::new_file(thumbnail_png, absolute_path).into_future(ctx.clone()).await;
                        thumbnail_path = Some(path);
                    }

                    let resource = LocalResource {
                        name: file_name,
                        size: file_size,
                        path: LocalResourcePath::PlatformIdentifier(identifier),
                        thumbnail_path,
                        r#type: selection.r#type
                    };

                    resource
                }
            };

            local_resources.push(local_resource);
        }

        ctx.send_event(AppEvent::Transfer(TransferEvent::UpdateLocalResources(local_resources)));
        ctx.request_from_shell(CoreOperation::Render).await;
    }
}
