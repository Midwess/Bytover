use serde::{Deserialize, Serialize};
use uniffi::Record;

use crate::entities::device::DeviceInfo;

#[derive(Debug, Serialize, Deserialize, Record, PartialEq, Eq, Clone)]
pub struct PeerViewModel {
    pub id: String,
    pub display_name: String,
    pub avatar_url: String,
    pub device: DeviceInfo
}
