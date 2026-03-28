use serde::{Deserialize, Serialize};

use crate::entities::peer::Peer;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum P2PConnectionState {
    NotConnected,
    Connecting,
    Connected,
    Failed(String)
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum TransferTarget {
    P2P {
        from_peer: Option<Peer>,
        connection_state: P2PConnectionState,
        #[serde(default)]
        signalling_key: Option<String>
    },
    Internet {
        to_emails: Vec<String>
    }
}

impl TransferTarget {
    pub fn is_connection_failed(&self) -> bool {
        let TransferTarget::P2P { connection_state, .. } = self else {
            return false
        };

        if matches!(connection_state, P2PConnectionState::Failed(_)) {
            return true
        }

        false
    }

    pub fn is_public(&self) -> bool {
        matches!(self, Self::Internet { .. })
    }

    pub fn is_peer(&self) -> bool {
        matches!(self, Self::P2P { .. })
    }

    pub fn is_connected(&self) -> bool {
        match self {
            TransferTarget::P2P { connection_state, .. } => {
                matches!(connection_state, P2PConnectionState::Connected)
            }
            TransferTarget::Internet { .. } => false
        }
    }

    pub fn is_connecting(&self) -> bool {
        match self {
            TransferTarget::P2P { connection_state, .. } => {
                matches!(connection_state, P2PConnectionState::Connecting)
            }
            TransferTarget::Internet { .. } => false
        }
    }

    pub fn is_failed(&self) -> bool {
        match self {
            TransferTarget::P2P { connection_state, .. } => {
                matches!(connection_state, P2PConnectionState::Failed(_))
            }
            TransferTarget::Internet { .. } => false
        }
    }

    pub fn connection_state(&self) -> Option<&P2PConnectionState> {
        match self {
            TransferTarget::P2P { connection_state, .. } => Some(connection_state),
            TransferTarget::Internet { .. } => None
        }
    }

    pub fn set_connection_state(&mut self, state: P2PConnectionState) {
        if let TransferTarget::P2P { connection_state, .. } = self {
            *connection_state = state;
        }
    }
}

impl TransferTarget {
    pub fn id(&self) -> String {
        match self {
            TransferTarget::P2P { from_peer, .. } => from_peer.as_ref().map(|p| p.id.clone()).unwrap_or_else(|| "unknown".to_string()),
            TransferTarget::Internet { .. } => "public".to_string()
        }
    }
}
