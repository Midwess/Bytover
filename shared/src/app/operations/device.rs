use std::future::Future;

use crux_core::{capability::Operation, Command};
use serde::{Deserialize, Serialize};

use crate::app::{modules::environment::DeviceInfo, AppRequestBuilder};

use super::{CoreOperation, CoreOperationOutput};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DeviceOperation {
    GetDeviceInfo
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DeviceOperationOutput {
    DeviceInfo(DeviceInfo)
}

impl Operation for DeviceOperation {
    type Output = DeviceOperationOutput;
}

impl DeviceOperation {
    pub fn get_device_info() -> AppRequestBuilder<impl Future<Output = DeviceInfo>> {
        Command::request_from_shell(CoreOperation::Device(DeviceOperation::GetDeviceInfo))
            .map(|output| match output {
                CoreOperationOutput::Device(DeviceOperationOutput::DeviceInfo(device_info)) => device_info,
                _ => panic!("Invalid output for DeviceOperation::GetDeviceInfo")
            })
    }
}