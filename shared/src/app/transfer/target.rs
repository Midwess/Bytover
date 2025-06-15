use serde::{Deserialize, Serialize};
use surreal_derive_plus::SurrealDerive;
use uniffi::Enum;

use crate::entities::peer::Peer;

#[derive(Debug, Enum, Serialize, Deserialize, Clone, PartialEq, Eq, SurrealDerive)]
pub enum TransferTarget {
    Nearby(Peer),
    Public { password: Option<String> }
}

impl TransferTarget {
    pub fn id(&self) -> String {
        match self {
            TransferTarget::Nearby(peer) => peer.id().to_string(),
            TransferTarget::Public { .. } => "public".to_string()
        }
    }
}
