use serde::{Deserialize, Serialize};
use uniffi::Record;

use crate::entities::device::DeviceInfo;

use super::avatar::AvatarViewModel;

#[derive(Debug, Serialize, Deserialize, Record, PartialEq, Clone)]
pub struct PeerViewModel {
    pub id: String,
    pub display_name: String,
    pub avatar: AvatarViewModel,
    pub device: DeviceInfo,
    pub transfer_progress: f64
}
