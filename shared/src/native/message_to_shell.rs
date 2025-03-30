use serde::{Deserialize, Serialize};
use uniffi::Enum;

use crate::entities::peer::Peer;

#[derive(Debug, Clone, Enum, PartialEq, Serialize, Deserialize)]
pub enum MessageToShell {
    NewPeer(Peer),
    PeerLeaved(Peer)
}
