use serde::{Deserialize, Serialize};

use crate::entities::peer::Peer;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PeerAvatarViewModel {
    pub name: String,
    pub avatar_url: String
}

impl From<&Peer> for PeerAvatarViewModel {
    fn from(peer: &Peer) -> Self {
        Self {
            name: peer.name.clone().unwrap_or_else(|| peer.device.name.clone()),
            avatar_url: peer.avatar_url.clone()
        }
    }
}
