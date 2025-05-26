use serde::{Deserialize, Serialize};
use surreal_derive_plus::SurrealDerive;
use uniffi::{Enum, Record};

use crate::app::operations::database::LocalResourceDatabaseOperation;
use crate::app::operations::local_storage::LocalStorageOperation;
use crate::app::AppCommandContext;

#[derive(Debug, PartialEq, Record, Serialize, Deserialize, Clone, SurrealDerive)]
pub struct LocalResource {
    pub order_id: u64,
    pub name: String,
    pub size: u64,
    pub path: LocalResourcePath,
    pub thumbnail_path: Option<LocalResourcePath>,
    pub r#type: ResourceType,
    #[surreal(default)]
    pub is_valid: bool
}

#[derive(Debug, PartialEq, Eq, Enum, Serialize, Deserialize, Clone, SurrealDerive)]
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

    pub fn as_string(&self) -> String {
        match self {
            LocalResourcePath::RelativePath { path, .. } => path.clone(),
            LocalResourcePath::PlatformIdentifier(identifier) => identifier.clone(),
            LocalResourcePath::AbsolutePath(path) => path.clone()
        }
    }
}

#[derive(Debug, PartialEq, Eq, Enum, Serialize, Deserialize, Clone, SurrealDerive)]
pub enum ResourceType {
    Image,
    Video,
    File,
    Folder
}

impl LocalResource {
    // Return true if value is updated
    pub async fn validate(&mut self, cmd: AppCommandContext) -> bool {
        let is_valid = LocalStorageOperation::is_file_exists(self.path.clone()).into_future(cmd.clone()).await;
        let is_changed = self.is_valid != is_valid;
        self.is_valid = is_valid;

        if is_changed && !is_valid {
            LocalResourceDatabaseOperation::remove(self.order_id).into_future(cmd.clone()).await;
        }

        is_changed
    }
}
