use std::future::Future;

use serde::{Deserialize, Serialize};

use crate::app::core::command::AppCommand;
use crate::app::AppRequestBuilder;
use crate::entities::device::DeviceInfo;
use crate::entities::local_resource::{LocalResourcePath, ResourceType};

use super::CoreOperationOutput;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GeoLocation {
    pub latitude: f64,
    pub longitude: f64
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DeviceOperation {
    GetDeviceInfo,
    GetGeoLocation,
    OpenSession(u64),
    Open(LocalResourcePath),
    LoadThumbnailPng {
        resource_type: ResourceType,
        path: LocalResourcePath,
        id: u64
    }
}

impl DeviceOperation {
    pub fn get_device_info() -> AppRequestBuilder<impl Future<Output = Option<DeviceInfo>>> {
        AppCommand::request_from_shell(Self::GetDeviceInfo).map(|output| output.option())
    }

    pub fn get_geo_location() -> AppRequestBuilder<impl Future<Output = Option<GeoLocation>>> {
        AppCommand::request_from_shell(Self::GetGeoLocation).map(|output| output.option())
    }

    pub fn load_thumbnail_png(
        resource_id: u64,
        path: LocalResourcePath,
        resource_type: ResourceType
    ) -> AppRequestBuilder<impl Future<Output = (Option<Vec<u8>>, Option<LocalResourcePath>)>> {
        AppCommand::request_from_shell(Self::LoadThumbnailPng {
            path,
            resource_type,
            id: resource_id
        }).map(|output| match output {
            CoreOperationOutput::ThumbnailPng(data) => (Some(data), None),
            CoreOperationOutput::LocalResourcePath(path) => (None, Some(path)),
            _ => (None, None)
        })
    }

    pub fn open_session(session_id: u64) -> AppRequestBuilder<impl Future<Output = ()>> {
        AppCommand::request_from_shell(DeviceOperation::OpenSession(session_id)).map(|_it| ())
    }

    pub fn open(path: LocalResourcePath) -> AppRequestBuilder<impl Future<Output = ()>> {
        AppCommand::request_from_shell(DeviceOperation::Open(path)).map(|_it| ())
    }
}
