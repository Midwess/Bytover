use std::future::Future;

use serde::{Deserialize, Serialize};

use crate::app::core::command::AppCommand;
use crate::app::operations::persistent::{LocalResourcePersistentOperationOutput, PersistentOperationOutput};
use crate::app::AppRequestBuilder;
use crate::entities::device::DeviceInfo;
use crate::entities::local_resource::LocalResourcePath;

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
    Open(OpenOperation),
    LoadThumbnailPng(LocalResourcePath)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OpenOperation {
    OpenSession(u64),
    Open(LocalResourcePath)
}

impl DeviceOperation {
    pub fn get_device_info() -> AppRequestBuilder<impl Future<Output = Option<DeviceInfo>>> {
        AppCommand::request_from_shell(Self::GetDeviceInfo).map(|output| output.option())
    }

    pub fn get_geo_location() -> AppRequestBuilder<impl Future<Output = Option<GeoLocation>>> {
        AppCommand::request_from_shell(Self::GetGeoLocation).map(|output| output.option())
    }

    pub fn load_thumbnail_png(
        path: LocalResourcePath
    ) -> AppRequestBuilder<impl Future<Output = (Option<Vec<u8>>, Option<LocalResourcePath>)>> {
        AppCommand::request_from_shell(Self::LoadThumbnailPng(path)).map(|output| match output {
            CoreOperationOutput::ThumbnailPng(data) => (Some(data), None),
            CoreOperationOutput::Persistent(PersistentOperationOutput::LocalResource(
                LocalResourcePersistentOperationOutput::AddThumbnail(path)
            )) => (None, Some(path)),
            _ => (None, None)
        })
    }
}

impl OpenOperation {
    pub fn open_session(session_id: u64) -> AppRequestBuilder<impl Future<Output = ()>> {
        AppCommand::request_from_shell(DeviceOperation::Open(OpenOperation::OpenSession(session_id))).map(|_it| ())
    }

    pub fn open(path: LocalResourcePath) -> AppRequestBuilder<impl Future<Output = ()>> {
        AppCommand::request_from_shell(DeviceOperation::Open(OpenOperation::Open(path))).map(|_it| ())
    }
}
