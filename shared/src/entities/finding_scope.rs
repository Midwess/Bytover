use chrono::Local;
use schema::devlog::rpc_signalling::server::ScopeState;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Eq, Serialize, Deserialize)]
pub struct FindingScope {
    scope_id: String,
    is_direct: bool,
    is_owner: bool,
    state: ScopeState,
    owner_peer_id: Option<String>
}

impl PartialEq for FindingScope {
    fn eq(&self, other: &Self) -> bool {
        self.scope_id.eq(&other.scope_id)
    }
}

impl FindingScope {
    pub fn new(request_scope: &str) -> Self {
        let (protocol, scope) = {
            let it = request_scope.split("://").collect::<Vec<_>>();
            if it.len() < 2 {
                ("".to_owned(), request_scope.to_owned())
            } else {
                (it[0].to_owned(), it[1].to_owned())
            }
        };

        let is_direct = protocol.contains("direct");
        let scope_id = scope.split(";").next().unwrap_or(&scope).to_owned();
        let is_owner = request_scope.split(";").any(|s| s.starts_with("owner"));

        Self {
            scope_id,
            is_direct,
            is_owner,
            state: ScopeState::Offline,
            owner_peer_id: None
        }
    }

    pub fn scope_id(&self) -> &str {
        &self.scope_id
    }

    pub fn is_direct(&self) -> bool {
        self.is_direct
    }

    pub fn is_owner(&self) -> bool {
        self.is_owner
    }

    pub fn is_online(&self) -> bool {
        self.state == ScopeState::Online
    }

    pub fn state(&self) -> ScopeState {
        self.state
    }

    pub fn update_state(&mut self, state: ScopeState) {
        self.state = state;
    }

    pub fn owner_peer_id(&self) -> Option<&str> {
        self.owner_peer_id.as_deref()
    }

    pub fn set_owner_peer_id(&mut self, peer_id: Option<String>) {
        self.owner_peer_id = peer_id;
    }

    pub fn has_owner(&self) -> bool {
        self.owner_peer_id.is_some()
    }

    pub fn from_string(s: String) -> Option<Self> {
        Some(FindingScope::new(&s))
    }

    pub fn as_string(&self) -> String {
        let protocol = if self.is_direct { "direct" } else { "" };
        let base = if protocol.is_empty() {
            self.scope_id.clone()
        } else {
            format!("{}://{}", protocol, self.scope_id)
        };

        if self.is_direct {
            let role = if self.is_owner { "owner" } else { "member" };
            format!("{};{}", base, role)
        } else {
            base
        }
    }

    #[allow(dead_code)]
    fn get_gmt_offset() -> i32 {
        let local_time = Local::now();
        let offset_seconds = local_time.offset().local_minus_utc();

        offset_seconds / 3600
    }
}

impl From<String> for FindingScope {
    fn from(s: String) -> Self {
        FindingScope::new(&s)
    }
}
