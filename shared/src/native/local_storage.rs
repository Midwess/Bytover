use core_services::local_storage::file_system::File;
use devlog_sdk::distributed_id::gen_id;

use crate::app::file_system::file::{LocalResource, LocalResourcePath, ResourceType};
use crate::app::operations::local_storage::{LocalStorageOperation, LocalStorageOperationOutput};

pub struct NativeLocalStorage {}

impl NativeLocalStorage {
    pub async fn handle(&self, effect: LocalStorageOperation) -> LocalStorageOperationOutput {
        match effect {
            LocalStorageOperation::NewFile { bytes, path } => {
                let created_file = File::new(Some(bytes), path).await.unwrap();
                let metadata = created_file.metadata().await.unwrap();
                let resource = LocalResource {
                    order_id: gen_id().await,
                    name: created_file.name,
                    size: metadata.size,
                    path: LocalResourcePath::AbsolutePath(created_file.path.to_string_lossy().to_string()),
                    thumbnail_path: None,
                    r#type: ResourceType::File,
                    is_valid: true
                };

                LocalStorageOperationOutput::NewFile(resource)
            }
            LocalStorageOperation::Copy { source, destination } => {
                let created_file = File::new(None, source).await.unwrap();
                let new_file = created_file.copy_to(destination).await.unwrap();
                let metadata = new_file.metadata().await.unwrap();
                let resource = LocalResource {
                    order_id: gen_id().await,
                    name: new_file.name,
                    size: metadata.size,
                    path: LocalResourcePath::AbsolutePath(new_file.path.to_string_lossy().to_string()),
                    thumbnail_path: None,
                    r#type: ResourceType::File,
                    is_valid: true
                };

                LocalStorageOperationOutput::Copy(resource)
            }
            LocalStorageOperation::Zip { source, destination } => {
                let created_file = File::new(None, source).await.unwrap();
                let new_file = created_file.zip(destination).await.unwrap();
                let metadata = new_file.metadata().await.unwrap();
                let resource = LocalResource {
                    order_id: gen_id().await,
                    name: new_file.name,
                    size: metadata.size,
                    path: LocalResourcePath::AbsolutePath(new_file.path.to_string_lossy().to_string()),
                    thumbnail_path: None,
                    r#type: ResourceType::File,
                    is_valid: true
                };

                LocalStorageOperationOutput::Zip(resource)
            }
            LocalStorageOperation::Get { path } => {
                let file = File::new(None, path).await.unwrap();
                let metadata = file.metadata().await.unwrap();
                let resource = LocalResource {
                    order_id: gen_id().await,
                    name: file.name.clone(),
                    size: metadata.size,
                    path: LocalResourcePath::AbsolutePath(file.path.to_string_lossy().to_string()),
                    thumbnail_path: None,
                    r#type: ResourceType::File,
                    is_valid: true
                };

                LocalStorageOperationOutput::Get(Some(resource))
            }
            LocalStorageOperation::IsFileExists { absolute_path } => {
                let file = File::existing(absolute_path).await;
                LocalStorageOperationOutput::IsFileExists(file.is_ok())
            }
            LocalStorageOperation::GetResourceType { absolute_path } => {
                let file_result = File::existing(&absolute_path).await;
                
                if let Ok(file) = file_result {
                    let metadata = file.metadata().await;
                    
                    if let Ok(metadata) = metadata {
                        if metadata.is_dir {
                            return LocalStorageOperationOutput::GetResourceType(Some(ResourceType::Folder));
                        } else {
                            return LocalStorageOperationOutput::GetResourceType(Some(ResourceType::File));
                        }
                    }
                }
                
                LocalStorageOperationOutput::GetResourceType(None)
            }
            _ => {
                panic!("Unsupported operation: {effect:?}")
            }
        }
    }
}
