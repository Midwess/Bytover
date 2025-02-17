use std::future::Future;

use crux_core::{capability::{CapabilityContext, Operation}, command, Command, Effect};
use serde::{Deserialize, Serialize};

use crate::app::{AppCommand, AppEvent, AppRequestBuilder};

use super::{CoreOperation, CoreOperationOutput};

/// This operation is used to access the local storage of device.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum LocalStorageOperation {
    GetWorkDirPath
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum LocalStorageOperationOutput {
    WorkDirPath(String)
}

impl Operation for LocalStorageOperation {
    type Output = LocalStorageOperationOutput;
}

impl LocalStorageOperation {
    pub fn get_work_dir_path_cmd() -> AppRequestBuilder<impl Future<Output = String>> {
        Command::request_from_shell(CoreOperation::LocalStorage(LocalStorageOperation::GetWorkDirPath))
            .map(|it| {
                match it {
                    CoreOperationOutput::LocalStorage(LocalStorageOperationOutput::WorkDirPath(path)) => path,
                    _ => panic!("Mismatch in response type, expected WorkDirPath, got {:?}", it),
                }
            })
    }
}