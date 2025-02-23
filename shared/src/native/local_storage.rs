use core_services::local_storage::file_system::File;

use crate::{app::operations::local_storage::{LocalStorageOperation, LocalStorageOperationOutput}, entities::file::{LocalResource, LocalResourcePath, ResourceType}};

pub struct NativeLocalStorage {}

impl NativeLocalStorage {
    pub async fn handle(&self, effect: LocalStorageOperation) -> LocalStorageOperationOutput {
        match effect {
            LocalStorageOperation::NewFile { bytes, path } => {
                let created_file = File::new(Some(bytes), path).await.unwrap();
                let metadata = created_file.metadata().await.unwrap();
                let resource = LocalResource {
                    name: created_file.name,
                    size: metadata.size,
                    path: LocalResourcePath::LocalPath(created_file.path.to_string_lossy().to_string()),
                    thumbnail_path: None,
                    r#type: ResourceType::File,
                };

                LocalStorageOperationOutput::NewFile(resource)
            }
            LocalStorageOperation::Copy { source, destination } => {
                let created_file = File::new(None, source).await.unwrap();
                let new_file = created_file.copy_to(destination).await.unwrap();
                let metadata = new_file.metadata().await.unwrap();
                let resource = LocalResource {
                    name: new_file.name,
                    size: metadata.size,
                    path: LocalResourcePath::LocalPath(new_file.path.to_string_lossy().to_string()),
                    thumbnail_path: None,
                    r#type: ResourceType::File,
                };

                LocalStorageOperationOutput::Copy(resource)
            }
            LocalStorageOperation::Zip { source, destination } => {
                let created_file = File::new(None, source).await.unwrap();
                let new_file = created_file.zip(destination).await.unwrap();
                let metadata = new_file.metadata().await.unwrap();
                let resource = LocalResource {
                    name: new_file.name,
                    size: metadata.size,
                    path: LocalResourcePath::LocalPath(new_file.path.to_string_lossy().to_string()),
                    thumbnail_path: None,
                    r#type: ResourceType::File,
                };

                LocalStorageOperationOutput::Zip(resource)
            }
            LocalStorageOperation::Get { path } => {
                let file = File::new(None, path).await.unwrap();
                let metadata = file.metadata().await.unwrap();
                let resource = LocalResource {
                    name: file.name.clone(),
                    size: metadata.size,
                    path: LocalResourcePath::LocalPath(file.path.to_string_lossy().to_string()),
                    thumbnail_path: None,
                    r#type: ResourceType::File,
                };

                LocalStorageOperationOutput::Get(Some(resource))
            }
            _ => {
                panic!("Unsupported operation: {:?}", effect)
            }
        }
    }
}
