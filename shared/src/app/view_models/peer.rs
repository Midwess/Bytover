use serde::{Deserialize, Serialize};

use crate::entities::device::DeviceInfo;
use crate::entities::peer::Peer;

use super::avatar::AvatarViewModel;

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct PeerViewModel {
    pub id: String,
    pub display_name: String,
    pub avatar: AvatarViewModel,
    pub device: DeviceInfo,
    pub transfer_progress: f64,
    pub display_upload_speed: Option<String>,
    pub display_download_speed: Option<String>
}

impl From<&Peer> for PeerViewModel {
    fn from(peer: &Peer) -> Self {
        Self {
            id: peer.id.clone(),
            display_name: peer.name.clone().unwrap_or(peer.device.name.clone()),
            avatar: AvatarViewModel::new(peer.avatar_url.clone()),
            device: peer.device.clone(),
            transfer_progress: 0.0,
            display_upload_speed: None,
            display_download_speed: None
        }
    }
}
