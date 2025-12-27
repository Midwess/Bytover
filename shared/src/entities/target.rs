use serde::{Deserialize, Serialize};

use crate::entities::peer::Peer;
use crate::entities::user::User;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum TransferTarget {
    P2P {
        from_peer: Option<Peer>,
        password: Option<String>,
        is_required_password: bool,
        signalling_key: String,
        scope: String
    },
    Internet {
        password: Option<String>,
        access_url: Option<String>,
        from_user: User,
        to_emails: Vec<String>,
        is_required_password: bool
    }
}

impl TransferTarget {
    pub fn is_public(&self) -> bool {
        matches!(self, Self::Internet { .. })
    }

    pub fn is_peer(&self) -> bool {
        matches!(self, Self::P2P { .. })
    }

    pub fn is_keyword_match(&self, keywords: &str) -> bool {
        if keywords.is_empty() {
            return true;
        }

        let TransferTarget::Internet {
            from_user,
            access_url: Some(access_url),
            ..
        } = self
        else {
            return false
        };

        let mut name: String = "".to_string();
        if let Ok(url) = url::Url::parse(access_url) {
            let Some(query) = url.query_pairs().find(|(key, _)| key == "session").map(|it| it.1.to_string()) else {
                return false
            };

            log::info!("Found query key session: {}", query);
            name = query;
        }

        from_user.name.to_lowercase() == keywords.to_lowercase() || name.to_lowercase() == keywords.to_lowercase()
    }
}

impl TransferTarget {
    pub fn id(&self) -> String {
        match self {
            TransferTarget::P2P { from_peer, .. } => from_peer.id().to_string(),
            TransferTarget::Internet { .. } => "public".to_string()
        }
    }
}
