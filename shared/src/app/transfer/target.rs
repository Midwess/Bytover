use serde::{Deserialize, Serialize};
use surreal_derive_plus::SurrealDerive;
use uniffi::Enum;

use crate::entities::peer::Peer;

#[derive(Debug, Enum, Serialize, Deserialize, Clone, PartialEq, Eq, SurrealDerive)]
pub enum TransferTarget {
    Nearby(Peer),
    Internet {
        password: Option<String>,
        // When it still loading it will be None
        access_url: Option<String>
    }
}

impl TransferTarget {
    pub fn id(&self) -> String {
        match self {
            TransferTarget::Nearby(peer) => peer.id().to_string(),
            TransferTarget::Internet { .. } => "public".to_string()
        }
    }
}
