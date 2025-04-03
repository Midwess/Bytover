use serde::{Deserialize, Serialize};
use surreal_derive_plus::SurrealDerive;
use uniffi::{Enum, Record};

#[derive(Debug, PartialEq, Eq, Record, Serialize, Deserialize, Clone, SurrealDerive)]
pub struct LocalResource {
    pub order_id: u64,
    pub name: String,
    pub size: u64,
    pub path: LocalResourcePath,
    pub thumbnail_path: Option<LocalResourcePath>,
    pub r#type: ResourceType
}

#[derive(Debug, PartialEq, Eq, Enum, Serialize, Deserialize, Clone, SurrealDerive)]
pub enum LocalResourcePath {
    LocalPath(String),
    PlatformIdentifier(String)
}

#[derive(Debug, PartialEq, Eq, Enum, Serialize, Deserialize, Clone, SurrealDerive)]
pub enum ResourceType {
    Image,
    Video,
    File,
    Folder,
    Other
}

impl LocalResourcePath {
    pub fn serialize(&self) -> String {
        match self {
            LocalResourcePath::LocalPath(path) => format!("local://{}", path),
            LocalResourcePath::PlatformIdentifier(identifier) => format!("platform://{}", identifier)
        }
    }

    pub fn deserialize(serialized: &str) -> Result<Self, String> {
        if serialized.starts_with("local://") {
            Ok(LocalResourcePath::LocalPath(serialized[7..].to_string()))
        } else if serialized.starts_with("platform://") {
            Ok(LocalResourcePath::PlatformIdentifier(serialized[10..].to_string()))
        } else {
            Err(format!("Invalid local resource path: {}", serialized))
        }
    }
}
