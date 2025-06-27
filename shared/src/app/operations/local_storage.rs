use std::future::Future;

use crux_core::capability::Operation;
use crux_core::Command;
use serde::{Deserialize, Serialize};
use uniffi::Enum;

use super::{CoreOperation, CoreOperationOutput};
use crate::app::file_system::file::{LocalResource, LocalResourcePath, ResourceType};
use crate::app::AppRequestBuilder;
use crate::errors::InputError;

/// This operation is used to access the local storage of device.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Enum)]
pub enum LocalStorageOperation {
    IsFileExists {
        path: LocalResourcePath
    },
    GetResourceType {
        path: LocalResourcePath
    },
    GetAbsolutePath(LocalResourcePath),
    LoadFileThumbnailPng(LocalResourcePath),
    Get {
        path: LocalResourcePath
    },
    NewThumbnail {
        png_bytes: Vec<u8>,
        resource_id: u64
    },
    Open {
        path: LocalResourcePath
    },
    OpenSession {
        session_id: u64
    },
    DeleteSession {
        session_id: u64
    },
    GenerateResourcePath {
        session_id: u64,
        resource_id: u64,
        resource_name: String
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Enum)]
pub enum LocalStorageOperationOutput {
    IsFileExists(bool),
    GetResourceType(Option<ResourceType>),
    Get(Option<LocalResource>),
    GetAbsolutePath(String),
    NewFile(LocalResource),
    Copy(LocalResource),
    LoadFileThumbnailPng(Option<Vec<u8>>),
    Delete(bool),
    BadRequest(InputError),
    GenerateResourcePath(Option<LocalResourcePath>)
}

impl Operation for LocalStorageOperation {
    type Output = LocalStorageOperationOutput;
}

impl LocalStorageOperation {
    pub fn new_thumbnail(bytes: Vec<u8>, resource_id: u64) -> AppRequestBuilder<impl Future<Output = LocalResource>> {
        Command::request_from_shell(CoreOperation::LocalStorage(LocalStorageOperation::NewThumbnail {
            png_bytes: bytes,
            resource_id
        }))
        .map(|it| match it {
            CoreOperationOutput::LocalStorage(LocalStorageOperationOutput::NewFile(resource)) => resource,
            _ => panic!("Mismatch in response type, expected NewFile, got {it:?}")
        })
    }

    pub fn get(path: LocalResourcePath) -> AppRequestBuilder<impl Future<Output = Option<LocalResource>>> {
        Command::request_from_shell(CoreOperation::LocalStorage(LocalStorageOperation::Get { path })).map(|it| match it {
            CoreOperationOutput::LocalStorage(LocalStorageOperationOutput::Get(resource)) => resource,
            _ => panic!("Mismatch in response type, expected Get, got {it:?}")
        })
    }

    pub fn load_file_thumbnail_png(path: LocalResourcePath) -> AppRequestBuilder<impl Future<Output = Option<Vec<u8>>>> {
        Command::request_from_shell(CoreOperation::LocalStorage(LocalStorageOperation::LoadFileThumbnailPng(path))).map(
            |it| match it {
                CoreOperationOutput::LocalStorage(LocalStorageOperationOutput::LoadFileThumbnailPng(thumbnail)) => thumbnail,
                _ => panic!("Mismatch in response type, expected LoadFileThumbnailPng, got {it:?}")
            }
        )
    }

    pub fn is_file_exists(path: LocalResourcePath) -> AppRequestBuilder<impl Future<Output = bool>> {
        Command::request_from_shell(CoreOperation::LocalStorage(LocalStorageOperation::IsFileExists { path })).map(|it| match it {
            CoreOperationOutput::LocalStorage(LocalStorageOperationOutput::IsFileExists(exists)) => exists,
            _ => panic!("Mismatch in response type, expected IsFileExists, got {it:?}")
        })
    }

    pub fn get_resource_type(path: LocalResourcePath) -> AppRequestBuilder<impl Future<Output = Option<ResourceType>>> {
        Command::request_from_shell(CoreOperation::LocalStorage(LocalStorageOperation::GetResourceType { path })).map(|it| match it {
            CoreOperationOutput::LocalStorage(LocalStorageOperationOutput::GetResourceType(resource_type)) => resource_type,
            _ => panic!("Mismatch in response type, expected GetResourceType, got {it:?}")
        })
    }

    pub fn open(path: LocalResourcePath) -> AppRequestBuilder<impl Future<Output = ()>> {
        Command::request_from_shell(CoreOperation::LocalStorage(LocalStorageOperation::Open { path })).map(|it| match it {
            CoreOperationOutput::Void => (),
            _ => panic!("Mismatch in response type, expected Void, got {it:?}")
        })
    }

    pub fn open_session(session_id: u64) -> AppRequestBuilder<impl Future<Output = ()>> {
        Command::request_from_shell(CoreOperation::LocalStorage(LocalStorageOperation::OpenSession { session_id })).map(
            |it| match it {
                CoreOperationOutput::Void => (),
                _ => panic!("Mismatch in response type, expected Void, got {it:?}")
            }
        )
    }

    pub fn delete_session(session_id: u64) -> AppRequestBuilder<impl Future<Output = bool>> {
        Command::request_from_shell(CoreOperation::LocalStorage(LocalStorageOperation::DeleteSession { session_id })).map(
            |it| match it {
                CoreOperationOutput::LocalStorage(LocalStorageOperationOutput::Delete(deleted)) => deleted,
                _ => panic!("Mismatch in response type, expected Delete, got {it:?}")
            }
        )
    }

    pub fn generate_resource_path(
        session_id: u64,
        resource_id: u64,
        resource_name: String
    ) -> AppRequestBuilder<impl Future<Output = Option<LocalResourcePath>>> {
        Command::request_from_shell(CoreOperation::LocalStorage(LocalStorageOperation::GenerateResourcePath {
            session_id,
            resource_id,
            resource_name
        }))
        .map(|it| match it {
            CoreOperationOutput::LocalStorage(LocalStorageOperationOutput::GenerateResourcePath(path)) => path,
            _ => panic!("Mismatch in response type, expected GenerateResourcePath, got {it:?}")
        })
    }
}
