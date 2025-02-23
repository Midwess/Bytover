use std::future::Future;

use crux_core::capability::Operation;
use crux_core::Command;
use serde::{Deserialize, Serialize};
use uniffi::Enum;

use crate::app::AppRequestBuilder;
use crate::entities::file::LocalResource;

use super::{CoreOperation, CoreOperationOutput};

/// This operation is used to access the local storage of device.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Enum)]
pub enum LocalStorageOperation {
    GetWorkDirPath,
    LoadFileSizeFromPlatformIdentifier(String),
    Get { path: String },
    NewFile { bytes: Vec<u8>, path: String },
    Copy { source: String, destination: String },
    Zip { source: String, destination: String }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Enum)]
pub enum LocalStorageOperationOutput {
    WorkDirPath(String),
    Get(Option<LocalResource>),
    NewFile(LocalResource),
    Copy(LocalResource),
    Zip(LocalResource),
    LoadFileSizeFromPlatformIdentifier(u64)
}

impl Operation for LocalStorageOperation {
    type Output = LocalStorageOperationOutput;
}

impl LocalStorageOperation {
    pub fn get_work_dir_path_cmd() -> AppRequestBuilder<impl Future<Output = String>> {
        Command::request_from_shell(CoreOperation::LocalStorage(LocalStorageOperation::GetWorkDirPath)).map(|it| {
            match it {
                CoreOperationOutput::LocalStorage(LocalStorageOperationOutput::WorkDirPath(path)) => path,
                _ => panic!("Mismatch in response type, expected WorkDirPath, got {:?}", it)
            }
        })
    }

    pub fn new_file(bytes: Vec<u8>, path: String) -> AppRequestBuilder<impl Future<Output = LocalResource>> {
        Command::request_from_shell(CoreOperation::LocalStorage(LocalStorageOperation::NewFile { bytes, path })).map(
            |it| match it {
                CoreOperationOutput::LocalStorage(LocalStorageOperationOutput::NewFile(resource)) => resource,
                _ => panic!("Mismatch in response type, expected NewFile, got {:?}", it)
            }
        )
    }

    pub fn copy(source: String, destination: String) -> AppRequestBuilder<impl Future<Output = LocalResource>> {
        Command::request_from_shell(CoreOperation::LocalStorage(LocalStorageOperation::Copy { source, destination }))
            .map(|it| match it {
                CoreOperationOutput::LocalStorage(LocalStorageOperationOutput::Copy(resource)) => resource,
                _ => panic!("Mismatch in response type, expected Copy, got {:?}", it)
            })
    }

    pub fn zip(source: String, destination: String) -> AppRequestBuilder<impl Future<Output = LocalResource>> {
        Command::request_from_shell(CoreOperation::LocalStorage(LocalStorageOperation::Zip { source, destination }))
            .map(|it| match it {
                CoreOperationOutput::LocalStorage(LocalStorageOperationOutput::Zip(resource)) => resource,
                _ => panic!("Mismatch in response type, expected Zip, got {:?}", it)
            })
    }

    pub fn get(path: String) -> AppRequestBuilder<impl Future<Output = Option<LocalResource>>> {
        Command::request_from_shell(CoreOperation::LocalStorage(LocalStorageOperation::Get { path })).map(|it| match it
        {
            CoreOperationOutput::LocalStorage(LocalStorageOperationOutput::Get(resource)) => resource,
            _ => panic!("Mismatch in response type, expected Get, got {:?}", it)
        })
    }

    pub fn load_file_size_from_platform_identifier(identifier: String) -> AppRequestBuilder<impl Future<Output = u64>> {
        Command::request_from_shell(CoreOperation::LocalStorage(LocalStorageOperation::LoadFileSizeFromPlatformIdentifier(identifier))).map(|it| match it {
            CoreOperationOutput::LocalStorage(LocalStorageOperationOutput::LoadFileSizeFromPlatformIdentifier(size)) => size,
            _ => panic!("Mismatch in response type, expected LoadFileSizeFromPlatformIdentifier, got {:?}", it)
        })
    }
}
