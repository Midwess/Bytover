use schema::devlog::bitbridge::TransferSessionMessage;
use serde::{Deserialize, Serialize};
use uniffi::Enum;

use crate::app::operations::CoreOperationOutput;
use crate::app::transfer::session::TransferProgress;
use crate::entities::peer::Peer;

#[derive(Debug, Clone, Enum, PartialEq, Serialize, Deserialize)]
pub enum MessageToShell {
    NewPeer(Peer),
    PeerLeaved(Peer),
    SessionRequest(TransferSessionMessage, Peer),
    SessionProgress(u64, TransferProgress),
    HandleResponse(u32, CoreOperationOutput)
}
