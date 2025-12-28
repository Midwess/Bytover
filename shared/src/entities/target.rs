use serde::{Deserialize, Serialize};

use crate::entities::peer::Peer;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum TransferTarget {
    P2P {
        from_peer: Option<Peer>,
        signalling_key: String,
        scope: String
    },
    Internet {
        to_emails: Vec<String>
    }
}

impl TransferTarget {
    pub fn is_public(&self) -> bool {
        matches!(self, Self::Internet { .. })
    }

    pub fn is_peer(&self) -> bool {
        matches!(self, Self::P2P { .. })
    }
}

impl TransferTarget {
    pub fn id(&self) -> String {
        match self {
            TransferTarget::P2P { from_peer, .. } => {
                from_peer.as_ref().map(|p| p.id().to_string()).unwrap_or_else(|| "unknown".to_string())
            }
            TransferTarget::Internet { .. } => "public".to_string()
        }
    }
}
