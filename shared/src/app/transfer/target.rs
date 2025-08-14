use serde::{Deserialize, Serialize};

use crate::entities::peer::Peer;
use crate::entities::user::User;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum TransferTarget {
    Nearby(Peer),
    Internet {
        password: Option<String>,
        access_url: Option<String>,
        from_user: User,
        to_email: Option<String>,
        is_required_password: bool
    }
}

impl TransferTarget {
    pub fn is_public(&self) -> bool {
        matches!(self, Self::Internet { .. })
    }

    pub fn is_peer(&self) -> bool {
        matches!(self, Self::Nearby(_))
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
