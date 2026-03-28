use crate::entities::device::DeviceInfo;
use schema::devlog::bitbridge::PeerMessage;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResourceReceivedPeer {
    pub id: String,
    pub avatar_url: String
}

impl From<&Peer> for ResourceReceivedPeer {
    fn from(peer: &Peer) -> Self {
        Self {
            id: peer.id.clone(),
            avatar_url: peer.avatar_url.clone()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Peer {
    pub id: String,
    pub name: Option<String>,
    pub avatar_url: String,
    pub email: Option<String>,
    pub device: DeviceInfo,
    pub user_id: Option<u64>,
    pub signalling_id: Option<String>
}

impl From<PeerMessage> for Peer {
    fn from(value: PeerMessage) -> Self {
        Self {
            id: value.peer_id,
            name: value.name.or_else(|| Some("Unknown".to_string())),
            avatar_url: value.avatar_url,
            email: value.email,
            device: value.device.into(),
            user_id: None,
            signalling_id: None
        }
    }
}

impl From<Peer> for PeerMessage {
    fn from(value: Peer) -> Self {
        Self {
            peer_id: value.id,
            name: value.name,
            avatar_url: value.avatar_url,
            email: value.email,
            device: value.device.into()
        }
    }
}
