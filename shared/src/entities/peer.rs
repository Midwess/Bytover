use schema::devlog::bitbridge::Peer as PeerSchema;
use serde::{Deserialize, Serialize};
use uniffi::Record;

use crate::entities::device::DeviceInfo;

// Peer is represent for the information that you want other
// people to know about
#[derive(Debug, Clone, Record, Serialize, Deserialize, PartialEq, Eq)]
pub struct Peer {
    pub id: String,
    pub name: Option<String>,
    pub avatar_url: String,
    pub email: Option<String>,
    pub device: DeviceInfo
}

impl Peer {
    pub fn id(&self) -> u128 {
        self.id.parse::<u128>().expect("Failed to parse peer id")
    }
}

impl From<PeerSchema> for Peer {
    fn from(value: PeerSchema) -> Self {
        Self {
            id: value.id,
            name: value.name,
            avatar_url: value.avatar_url,
            email: value.email,
            device: value.device.into()
        }
    }
}

impl From<Peer> for PeerSchema {
    fn from(value: Peer) -> Self {
        Self {
            id: value.id,
            name: value.name,
            avatar_url: value.avatar_url,
            email: value.email,
            device: value.device.into()
        }
    }
}
