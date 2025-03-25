use std::future::Future;

use crux_core::capability::Operation;
use crux_core::Command;
use serde::{Deserialize, Serialize};
use uniffi::Record;

use crate::app::modules::environment::DeviceInfo;
use crate::app::AppRequestBuilder;

use super::{CoreOperation, CoreOperationOutput};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Record)]
pub struct GeoLocation {
    pub latitude: f64,
    pub longitude: f64
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DeviceOperation {
    GetDeviceInfo,
    GetGeoLocation
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DeviceOperationOutput {
    DeviceInfo(DeviceInfo),
    GetGeoLocation(Option<GeoLocation>)
}

impl Operation for DeviceOperation {
    type Output = DeviceOperationOutput;
}

impl DeviceOperation {
    pub fn get_device_info() -> AppRequestBuilder<impl Future<Output = DeviceInfo>> {
        Command::request_from_shell(CoreOperation::Device(DeviceOperation::GetDeviceInfo)).map(|output| match output {
            CoreOperationOutput::Device(DeviceOperationOutput::DeviceInfo(device_info)) => device_info,
            _ => panic!("Invalid output for DeviceOperation::GetDeviceInfo")
        })
    }

    pub fn get_geo_location() -> AppRequestBuilder<impl Future<Output = Option<GeoLocation>>> {
        Command::request_from_shell(CoreOperation::Device(DeviceOperation::GetGeoLocation)).map(|output| match output {
            CoreOperationOutput::Device(DeviceOperationOutput::GetGeoLocation(geo_location)) => geo_location,
            _ => None
        })
    }
}
