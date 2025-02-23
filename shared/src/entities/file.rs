use serde::{Deserialize, Serialize};
use surreal_derive_plus::SurrealDerive;
use uniffi::{Enum, Record};

#[derive(Debug, PartialEq, Eq, Record, Serialize, Deserialize, Clone, SurrealDerive)]
pub struct LocalResource {
    pub name: String,
    pub size: u64,
    pub path: LocalResourcePath,
    pub thumbnail_path: Option<String>,
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
