use schema::value::device::{DeviceType, RegisteringDevice};
use schema::value::platform::Platform;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeviceInfo {
    pub platform: Platform,
    pub name: String,
    pub unique_id: String,
    pub device_type: DeviceType,
    pub url: String,
}

impl From<RegisteringDevice> for DeviceInfo {
    fn from(value: RegisteringDevice) -> Self {
        Self {
            platform: Platform::try_from(value.platform).unwrap_or_default(),
            name: value.device_name,
            unique_id: value.device_unique_key,
            device_type: DeviceType::try_from(value.device_type).unwrap_or_default(),
            url: value.url,
        }
    }
}

impl From<DeviceInfo> for RegisteringDevice {
    fn from(value: DeviceInfo) -> Self {
        Self {
            platform: value.platform as i32,
            device_name: value.name,
            device_unique_key: value.unique_id,
            device_type: value.device_type as i32,
            url: value.url,
        }
    }
}
