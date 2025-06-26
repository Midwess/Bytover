use std::path::PathBuf;

use core_services::local_storage::file_system::{File, Folder};
use devlog_sdk::local_id_generator::gen_id;
use shared::app::file_system::file::{LocalResource, LocalResourcePath, ResourceType};
use shared::app::file_system::workdir::WorkDir;
use shared::app::operations::local_storage::{LocalStorageOperation, LocalStorageOperationOutput};
use shared::errors::InputError;

pub struct NativeLocalStorage {
    pub workdir: WorkDir
}

impl NativeLocalStorage {
    pub async fn handle(&self, effect: LocalStorageOperation) -> LocalStorageOperationOutput {
        match effect {
            LocalStorageOperation::NewThumbnail { png_bytes, resource_id } => {
                let path = self.workdir.thumbnails(resource_id);
                let created_file = File::new(Some(png_bytes), path).await.unwrap();
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
            LocalStorageOperation::Get { path } => {
                let LocalResourcePath::AbsolutePath(path) = path else {
                    return LocalStorageOperationOutput::BadRequest(InputError::ExpectedAnAbsolutePath(path.clone()))
                };

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
            LocalStorageOperation::IsFileExists { path } => {
                let LocalResourcePath::AbsolutePath(absolute_path) = path else {
                    return LocalStorageOperationOutput::IsFileExists(false);
                };

                let file = File::existing(absolute_path).await;
                LocalStorageOperationOutput::IsFileExists(file.is_ok())
            }
            LocalStorageOperation::GetResourceType { path } => {
                let LocalResourcePath::AbsolutePath(absolute_path) = path else {
                    return LocalStorageOperationOutput::GetResourceType(None);
                };

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
            LocalStorageOperation::DeleteSession { session_id } => {
                let path = self.workdir.session_folder(session_id);

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
