use chrono::Local;
use serde::{Deserialize, Serialize};
use uniffi::Enum;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Enum)]
pub enum FindingScope {
    Global(String),
    Local(String)
}

impl FindingScope {
    pub fn from_string(s: String) -> Option<Self> {
        let parts = s.split(':').collect::<Vec<&str>>();
        if parts.len() < 2 {
            return None;
        }

        let Some(scope_key) = parts[1].split('-').next() else {
            return None;
        };

        if parts[0] == "public" {
            return Some(FindingScope::Global(scope_key.to_string()));
        } else if parts[0] == "local" {
            return Some(FindingScope::Local(scope_key.to_string()));
        }

        None
    }

    pub fn as_string(&self) -> String {
        let gmt_offset = Self::get_gmt_offset();
        match self {
            FindingScope::Global(content) => format!("public:{}", content),
            FindingScope::Local(content) => format!("local:{}-gmt{}", content, gmt_offset),
        }
    }

    fn get_gmt_offset() -> i32 {
        let local_time = Local::now();
        let offset_seconds = local_time.offset().local_minus_utc();
        let offset_hours = offset_seconds / 3600;

        offset_hours
    }

    pub fn is_local(&self) -> bool {
        matches!(self, FindingScope::Local(_))
    }

    pub fn is_global(&self) -> bool {
        matches!(self, FindingScope::Global(_))
    }
}

impl From<String> for FindingScope {
    fn from(s: String) -> Self {
        let parts = s.split(':').collect::<Vec<&str>>();
        if parts[0] == "public" {
            FindingScope::Global(parts[1].to_string())
        } else {
            FindingScope::Local(parts[1].to_string())
        }
    }
}
