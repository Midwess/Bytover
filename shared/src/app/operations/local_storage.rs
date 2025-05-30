use std::future::Future;

use crux_core::capability::Operation;
use crux_core::Command;
use serde::{Deserialize, Serialize};
use uniffi::Enum;

use crate::app::file_system::file::{LocalResource, LocalResourcePath, ResourceType};
use crate::app::file_system::workdir::WorkDir;
use crate::app::AppRequestBuilder;

use super::{CoreOperation, CoreOperationOutput};

/// This operation is used to access the local storage of device.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Enum)]
pub enum LocalStorageOperation {
    GetWorkDirPath,
    IsFileExists { absolute_path: String },
    GetResourceType { absolute_path: String },
    GetAbsolutePath(LocalResourcePath),
    LoadFileThumbnailPng(LocalResourcePath),
    Get { path: String },
    NewFile { bytes: Vec<u8>, path: String },
    Copy { source: String, destination: String },
    Open { path: LocalResourcePath },
    Delete { path: LocalResourcePath }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Enum)]
pub enum LocalStorageOperationOutput {
    WorkDirPath(WorkDir),
    IsFileExists(bool),
    GetResourceType(Option<ResourceType>),
    Get(Option<LocalResource>),
    GetAbsolutePath(String),
    NewFile(LocalResource),
    Copy(LocalResource),
    LoadFileThumbnailPng(Option<Vec<u8>>),
    Delete(bool)
}

impl Operation for LocalStorageOperation {
    type Output = LocalStorageOperationOutput;
}

impl LocalStorageOperation {
    pub fn get_work_dir_path_cmd() -> AppRequestBuilder<impl Future<Output = WorkDir>> {
        Command::request_from_shell(CoreOperation::LocalStorage(LocalStorageOperation::GetWorkDirPath)).map(|it| match it {
            CoreOperationOutput::LocalStorage(LocalStorageOperationOutput::WorkDirPath(path)) => path,
            _ => panic!("Mismatch in response type, expected WorkDirPath, got {it:?}")
        })
    }

    pub fn new_file(bytes: Vec<u8>, path: String) -> AppRequestBuilder<impl Future<Output = LocalResource>> {
        Command::request_from_shell(CoreOperation::LocalStorage(LocalStorageOperation::NewFile { bytes, path })).map(|it| match it {
            CoreOperationOutput::LocalStorage(LocalStorageOperationOutput::NewFile(resource)) => resource,
            _ => panic!("Mismatch in response type, expected NewFile, got {it:?}")
        })
    }

    pub fn copy(source: String, destination: String) -> AppRequestBuilder<impl Future<Output = LocalResource>> {
        Command::request_from_shell(CoreOperation::LocalStorage(LocalStorageOperation::Copy { source, destination })).map(
            |it| match it {
                CoreOperationOutput::LocalStorage(LocalStorageOperationOutput::Copy(resource)) => resource,
                _ => panic!("Mismatch in response type, expected Copy, got {it:?}")
            }
        )
    }

    pub fn get(path: String) -> AppRequestBuilder<impl Future<Output = Option<LocalResource>>> {
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

    pub fn get_absolute_path(path: LocalResourcePath) -> AppRequestBuilder<impl Future<Output = String>> {
        Command::request_from_shell(CoreOperation::LocalStorage(LocalStorageOperation::GetAbsolutePath(path))).map(|it| match it {
            CoreOperationOutput::LocalStorage(LocalStorageOperationOutput::GetAbsolutePath(path)) => path,
            _ => panic!("Mismatch in response type, expected GetAbsolutePath, got {it:?}")
        })
    }

    pub fn is_file_exists(path: LocalResourcePath) -> AppRequestBuilder<impl Future<Output = bool>> {
        Self::get_absolute_path(path)
            .then_request(|it| {
                Command::request_from_shell(CoreOperation::LocalStorage(LocalStorageOperation::IsFileExists {
                    absolute_path: it
                }))
            })
            .map(|it| match it {
                CoreOperationOutput::LocalStorage(LocalStorageOperationOutput::IsFileExists(exists)) => exists,
                _ => panic!("Mismatch in response type, expected IsFileExists, got {it:?}")
            })
    }

    pub fn get_resource_type(path: LocalResourcePath) -> AppRequestBuilder<impl Future<Output = Option<ResourceType>>> {
        Self::get_absolute_path(path)
            .then_request(|absolute_path| {
                Command::request_from_shell(CoreOperation::LocalStorage(LocalStorageOperation::GetResourceType {
                    absolute_path
                }))
            })
            .map(|it| match it {
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

    pub fn delete(path: LocalResourcePath) -> AppRequestBuilder<impl Future<Output = bool>> {
        Command::request_from_shell(CoreOperation::LocalStorage(LocalStorageOperation::Delete { path })).map(|it| match it {
            CoreOperationOutput::LocalStorage(LocalStorageOperationOutput::Delete(deleted)) => deleted,
            _ => panic!("Mismatch in response type, expected Delete, got {it:?}")
        })
    }
}
