use serde::{Deserialize, Serialize};
use uniffi::Enum;

use crate::entities::{peer::Peer, user::User};

#[derive(Debug, Clone, Enum, PartialEq, Serialize, Deserialize)]
pub enum MessageToShell {
    NewPeer(Peer),
    PeerLeaved(Peer)
}
