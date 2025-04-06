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

impl LocalResource {
    pub fn identifer(&self, session_id: u64) -> String {
        let path = match &self.path {
            LocalResourcePath::LocalPath(path) => path.clone(),
            LocalResourcePath::PlatformIdentifier(identifier) => identifier.clone()
        };

        let file_name = path.split("/").last().unwrap_or("").to_string();
        format!("{}-{}-{}-{}", session_id, self.order_id, file_name, self.size)
    }

    pub fn read_identifier(identifier: String) -> Result<(u64, u64, String, u64), String> {
        let parts = identifier.split("-").collect::<Vec<&str>>();
        if parts.len() != 4 {
            return Err(format!("Invalid identifier: {}", identifier));
        }

        Ok((
            parts[0].parse::<u64>().unwrap(),
            parts[1].parse::<u64>().unwrap(),
            parts[2].to_string(),
            parts[3].parse::<u64>().unwrap()
        ))
    }
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
