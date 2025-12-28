use chrono::Local;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Eq, Serialize, Deserialize)]
pub struct FindingScope {
    scope_id: String,
    is_direct: bool,
    is_owner: bool,
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

    pub fn scope_id(&self) -> &str {
        &self.scope_id
    }

    pub fn is_direct(&self) -> bool {
        self.is_direct
    }

    pub fn is_owner(&self) -> bool {
        self.is_owner
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
            if self.is_owner {
                format!("{};owner", base)
            } else {
                format!("{};member", base)
            }
        }
        else {
            base
        }
    }

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
