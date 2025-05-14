use serde::{Deserialize, Serialize};
use surreal_derive_plus::SurrealDerive;
use uniffi::{Enum, Record};

use crate::app::operations::database::LocalResourceDatabaseOperation;
use crate::app::operations::local_storage::LocalStorageOperation;
use crate::app::AppCommandContext;

#[derive(Debug, PartialEq, Eq, Record, Serialize, Deserialize, Clone, SurrealDerive)]
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
    RelativePath(String),
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

    pub fn from_absolute_to_relative(path: String, workdir: String) -> Self {
        let relative_path = path.replace(&workdir, "");
        LocalResourcePath::RelativePath(relative_path)
    }

    pub fn from_relative_to_absolute(path: String, workdir: String) -> Self {
        let absolute_path = format!("{workdir}/{path}");
        LocalResourcePath::AbsolutePath(absolute_path)
    }

    pub fn as_string(&self) -> String {
        match self {
            LocalResourcePath::RelativePath(path) => path.clone(),
            LocalResourcePath::PlatformIdentifier(identifier) => identifier.clone(),
            LocalResourcePath::AbsolutePath(path) => path.clone()
        }
    }
}

#[derive(Debug, PartialEq, Eq, Enum, Serialize, Deserialize, Clone, SurrealDerive)]
pub enum ResourceType {
    Image,
    Video,
    File
}

impl LocalResource {
    pub fn identifer(&self, session_id: u64) -> String {
        let path = match &self.path {
            LocalResourcePath::RelativePath(path) => path.clone(),
            LocalResourcePath::PlatformIdentifier(identifier) => identifier.clone(),
            LocalResourcePath::AbsolutePath(path) => path.clone()
        };

        let file_name = path.split("/").last().unwrap_or("").to_string();
        let ascii_file_name = Self::filename_to_ascii(&file_name);
        format!("{}-{}-{}-{}", session_id, self.order_id, ascii_file_name, self.size)
    }

    pub fn read_identifier(identifier: String) -> Result<(u64, u64, String, u64), String> {
        let parts = identifier.split("-").collect::<Vec<&str>>();
        if parts.len() != 4 {
            return Err(format!("Invalid identifier: {identifier}"));
        }

        let original_filename = Self::ascii_to_filename(parts[2]);

        Ok((
            parts[0].parse::<u64>().unwrap(),
            parts[1].parse::<u64>().unwrap(),
            original_filename,
            parts[3].parse::<u64>().unwrap()
        ))
    }

    fn filename_to_ascii(filename: &str) -> String {
        filename.chars().map(|c| format!("{:03}", c as u32)).collect()
    }

    fn ascii_to_filename(ascii: &str) -> String {
        let mut result = String::new();
        let mut i = 0;

        while i + 2 < ascii.len() {
            if let Ok(code) = ascii[i..i + 3].parse::<u32>() {
                if let Some(c) = std::char::from_u32(code) {
                    result.push(c);
                }
            }
            i += 3;
        }

        result
    }

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
