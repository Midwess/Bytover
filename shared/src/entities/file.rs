use serde::{Deserialize, Serialize};
use uniffi::{Enum, Record};

#[derive(Debug, PartialEq, Eq, Record, Serialize, Deserialize, Clone)]
pub struct LocalResource {
    md5: String,
    name: String,
    size: u64,
    path: String,
    thumbnail_path: Option<String>,
    r#type: ResourceType
}

#[derive(Debug, PartialEq, Eq, Enum, Serialize, Deserialize, Clone)]
pub enum ResourceType {
    Image,
    Video,
    File,
    Folder,
    Other
}
