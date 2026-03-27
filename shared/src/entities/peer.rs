use schema::devlog::bitbridge::PeerMessage;
use serde::{Deserialize, Serialize};

use crate::entities::device::DeviceInfo;
use crate::entities::target::TransferTarget;
use crate::entities::transfer_session::TransferSession;

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

// Peer is represent for the information that you want other
// people to know about
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Peer {
    pub id: String,
    pub name: Option<String>,
    pub avatar_url: String,
    pub email: Option<String>,
    pub device: DeviceInfo
}

impl Peer {
    pub fn compute_id(device_unique_id: &str, user_id: u64) -> String {
        format!("{device_unique_id}:{user_id}")
    }

    pub fn is_owned(&self, session: &TransferSession) -> bool {
        let TransferTarget::P2P { from_peer, .. } = &session.target else {
            return false;
        };

        from_peer.as_ref().map(|p| p.id.as_str() == self.id.as_str()).unwrap_or(false)
    }

    pub fn is_member(&self, _session: &TransferSession) -> bool {
        false
    }
}

impl From<PeerMessage> for Peer {
    fn from(value: PeerMessage) -> Self {
        Self {
            id: value.peer_id,
            name: value.name.or_else(|| Some("Unknown".to_string())),
            avatar_url: value.avatar_url,
            email: value.email,
            device: value.device.into()
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