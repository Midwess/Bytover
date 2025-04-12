use serde::{Deserialize, Serialize};
use uniffi::Record;

use crate::entities::{device::DeviceInfo, peer::Peer};

use super::avatar::AvatarViewModel;

#[derive(Debug, Serialize, Deserialize, Record, PartialEq, Clone)]
pub struct PeerViewModel {
    pub id: String,
    pub display_name: String,
    pub avatar: AvatarViewModel,
    pub device: DeviceInfo,
    pub transfer_progress: f64
}

impl From<&Peer> for PeerViewModel {
    fn from(peer: &Peer) -> Self {
        Self {
            id: peer.id.clone(),
            display_name: peer.name.clone().unwrap_or(peer.device.name.clone()),
            avatar: AvatarViewModel::new(peer.avatar_url.clone()),
            device: peer.device.clone(),
            transfer_progress: 0.0
        }
    }
}