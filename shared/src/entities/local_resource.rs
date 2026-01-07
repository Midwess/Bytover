use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct LocalResource {
    pub order_id: u64,
    pub name: String,
    pub size: u64,
    pub path: LocalResourcePath,
    pub thumbnail_path: Option<LocalResourcePath>,
    pub r#type: ResourceType,
    pub shelf_id: u64
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, Hash)]
pub enum LocalResourcePath {
    // Relative from the workdir of application
    RelativePath { path: String, is_private: bool },
    // Only the platform know how to get the absolute path
    PlatformIdentifier(String),
    // Absolute path on the device
    AbsolutePath(String)
}

impl LocalResourcePath {
    pub fn disk_path(&self) -> Option<String> {
        match self {
            LocalResourcePath::AbsolutePath(path) => Some(path.clone()),
            _ => None
        }
    }

    pub fn name(&self) -> Option<String> {
        self.as_string().split('/').next_back().map(|it| it.to_string())
    }

    pub fn extension(&self) -> Option<String> {
        self.as_string().split('.').last().map(|it| it.to_string())
    }

    pub fn as_string(&self) -> String {
        match self {
            LocalResourcePath::RelativePath { path, .. } => path.clone(),
            LocalResourcePath::PlatformIdentifier(identifier) => identifier.clone(),
            LocalResourcePath::AbsolutePath(path) => path.clone()
        }
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub enum ResourceType {
    Image,
    Video,
    File,
    Folder
}
