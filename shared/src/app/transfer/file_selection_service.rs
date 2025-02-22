use serde::{Deserialize, Serialize};
use uniffi::{Enum, Record};

use crate::{app::{modules::transfer::TransferEvent, operations::local_storage::LocalStorageOperation, AppCommandContext, AppEvent}, entities::{file::{LocalResource, ResourceType}, transfer::TransferSession}};

#[derive(Debug, PartialEq, Eq, Record, Serialize, Deserialize, Clone)]
pub struct ResourceSelection {
    // A unique identifier to know that the file is selected or not
    selection_id: String,
    data: ResourceSelectionData,
    r#type: ResourceType,
    name: String
}

#[derive(Debug, PartialEq, Eq, Enum, Serialize, Deserialize, Clone)]
pub enum ResourceSelectionData {
    Bytes(Vec<u8>),
    LocalPath(String)
}

pub struct ResourceTransferSelectionService {}

impl ResourceTransferSelectionService {
    pub async fn add_resource(
        &self, 
        ctx: AppCommandContext,
        selection: ResourceSelection
    ) {
        let local_resource = match selection.data {
            ResourceSelectionData::Bytes(bytes) => {
                LocalStorageOperation::new_file(bytes, selection.name).into_future(ctx.clone()).await
            }
            ResourceSelectionData::LocalPath(path) => {
                let resource = LocalStorageOperation::get(path).into_future(ctx.clone()).await;
                if resource.is_none() {
                    panic!("Resource not found")
                }

                resource.unwrap()
            }
        };

        ctx.send_event(AppEvent::Transfer(TransferEvent::AddResource(local_resource)));
    }
}
