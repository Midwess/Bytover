use std::future::Future;

use crux_core::capability::Operation;
use crux_core::Command;
use serde::{Deserialize, Serialize};
use uniffi::{Enum, Record};

use crate::app::file_system::file::LocalResourcePath;
use crate::app::AppRequestBuilder;
use crate::entities::device::DeviceInfo;

use super::{CoreOperation, CoreOperationOutput};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Record)]
pub struct GeoLocation {
    pub latitude: f64,
    pub longitude: f64
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Enum)]
pub enum DeviceOperation {
    GetDeviceInfo,
    GetGeoLocation,
    Open(OpenOperation),
    LoadThumbnailPng(LocalResourcePath)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Enum)]
pub enum DeviceOperationOutput {
    DeviceInfo(DeviceInfo),
    GetGeoLocation(Option<GeoLocation>),
    LoadThumbnailPng(Option<Vec<u8>>)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Enum)]
pub enum OpenOperation {
    OpenSession(u64),
    Open(LocalResourcePath)
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

    pub fn load_thumbnail_png(path: LocalResourcePath) -> AppRequestBuilder<impl Future<Output = Option<Vec<u8>>>> {
        Command::request_from_shell(CoreOperation::Device(DeviceOperation::LoadThumbnailPng(path))).map(|output| match output {
            CoreOperationOutput::Device(DeviceOperationOutput::LoadThumbnailPng(path)) => path,
            _ => None
        })
    }
}

impl OpenOperation {
    pub fn open_session(session_id: u64) -> AppRequestBuilder<impl Future<Output = ()>> {
        Command::request_from_shell(CoreOperation::Device(DeviceOperation::Open(OpenOperation::OpenSession(
            session_id
        ))))
        .map(|it| match it {
            CoreOperationOutput::Void => (),
            _ => panic!("Invalid output for DeviceOperation::OpenSession")
        })
    }

    pub fn open(path: LocalResourcePath) -> AppRequestBuilder<impl Future<Output = ()>> {
        Command::request_from_shell(CoreOperation::Device(DeviceOperation::Open(OpenOperation::Open(path)))).map(|it| match it {
            CoreOperationOutput::Void => (),
            _ => panic!("Invalid output for DeviceOperation::Open")
        })
    }
}
