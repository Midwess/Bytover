use matchbox_protocol::PeerId;
use schema::devlog::bitbridge::PeerMessage;
use serde::{Deserialize, Serialize};

use crate::entities::device::DeviceInfo;
use crate::entities::target::TransferTarget;
use crate::entities::transfer_session::TransferSession;

// Peer is represent for the information that you want other
// people to know about
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Peer {
    pub id: String,
    pub name: Option<String>,
    pub avatar_url: String,
    pub email: Option<String>,
    pub device: DeviceInfo,
    pub scopes: Vec<Scope>
}

impl Peer {
    pub fn id(&self) -> uuid::Uuid {
        // The id is always be String with uuid format, so we can unwrap safely
        self.id.clone().parse().unwrap_or_default()
    }

    pub fn peer_id(&self) -> PeerId {
        self.id().into()
    }

    pub fn owned_scopes(&self) -> Vec<&Scope> {
        self.scopes.iter().filter(|it| it.is_owner).collect::<Vec<_>>()
    }

    pub fn member_scopes(&self) -> Vec<&Scope> {
        self.scopes.iter().filter(|it| !it.is_owner).collect::<Vec<_>>()
    }

    pub fn is_owned(&self, session: &TransferSession) -> bool {
        let TransferTarget::P2P {
            scope,
            ..
        } = &session.target else {
            return false;
        };

        self.owned_scopes().iter().any(|it| it.scope_id.eq(&scope))
    }

    pub fn is_member(&self, session: &TransferSession) -> bool {
        let TransferTarget::P2P {
            scope,
            ..
        } = &session.target else {
            return false;
        };

        self.member_scopes().iter().any(|it| it.scope_id.eq(&scope))
    }
}

impl From<PeerMessage> for Peer {
    fn from(value: PeerMessage) -> Self {
        Self {
            id: value.peer_id,
            name: value.name.or_else(|| Some("Unknown".to_string())),
            avatar_url: value.avatar_url,
            email: value.email,
            device: value.device.into(),
            scopes: vec![]
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

#[derive(Clone, Debug, Eq)]
pub struct Scope {
    scope_id: String,
    is_direct: bool,
    is_owner: bool,
}

impl PartialEq for Scope {
    fn eq(&self, other: &Self) -> bool {
        self.scope_id.eq(&other.scope_id)
    }
}

impl Scope {
    fn new(request_scope: &str) -> Self {
        let (protocol, scope) = {
            let it = request_scope.split("://").collect::<Vec<_>>();
            if it.len() < 2 {
                ("".to_owned(), request_scope.to_owned())
            }
            else {
                (it[0].to_owned(), it[1].to_owned())
            }
        };

        let is_direct = protocol.contains("direct");
        let scope_id = scope.split(";").next().unwrap_or(&scope).to_owned();
        let is_owner = request_scope.split(";").any(|s| s.starts_with("owner"));

        Self { scope_id, is_direct, is_owner }
    }
}