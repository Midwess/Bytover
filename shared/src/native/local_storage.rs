use std::path::PathBuf;

use core_services::local_storage::file_system::{File, Folder};
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
            LocalStorageOperation::Get { path } => {
                let path_buf = PathBuf::from(path.clone());
                if path_buf.is_dir() {
                    let folder = Folder::new(path_buf).await.unwrap();
                    let resource = LocalResource {
                        order_id: gen_id().await,
                        name: folder.name.clone(),
                        size: folder.calculate_total_size().await.unwrap_or_default(),
                        path: LocalResourcePath::AbsolutePath(folder.path.to_string_lossy().to_string()),
                        thumbnail_path: None,
                        r#type: ResourceType::Folder,
                        is_valid: true
                    };

                    return LocalStorageOperationOutput::Get(Some(resource));
                } else if path_buf.is_symlink() {
                    return LocalStorageOperationOutput::Get(None);
                }

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
                            let mime_type = mime_guess::from_path(&file.path).first_or_octet_stream();
                            let resource_type = if mime_type.type_() == mime_guess::mime::IMAGE {
                                ResourceType::Image
                            } else if mime_type.type_() == mime_guess::mime::VIDEO {
                                ResourceType::Video
                            } else {
                                ResourceType::File
                            };
                            return LocalStorageOperationOutput::GetResourceType(Some(resource_type));
                        }
                    }
                }

                LocalStorageOperationOutput::GetResourceType(None)
            }
            LocalStorageOperation::Delete { path } => {
                let LocalResourcePath::AbsolutePath(path) = path else {
                    return LocalStorageOperationOutput::Delete(false);
                };

                let path_buf = PathBuf::from(path);
                if path_buf.is_dir() {
                    let folder = Folder::new(path_buf).await.unwrap();
                    let is_deleted = folder.delete().await.is_ok();
                    return LocalStorageOperationOutput::Delete(is_deleted);
                } else if path_buf.is_file() {
                    let file = File::existing(path_buf).await;
                    if let Ok(file) = file {
                        let is_deleted = file.delete().await.is_ok();
                        return LocalStorageOperationOutput::Delete(is_deleted);
                    }
                }

                LocalStorageOperationOutput::Delete(false)
            }
            _ => {
                panic!("Unsupported operation: {effect:?}")
            }
        }
    }
}
