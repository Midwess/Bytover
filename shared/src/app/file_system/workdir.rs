use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uniffi::Record;

use super::file::LocalResourcePath;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Record)]
pub struct WorkDir {
    private_path: String,
    public_path: String
}

impl WorkDir {
    pub fn new(private_path: String, public_path: String) -> Self {
        Self {
            private_path: Self::normalize_path(private_path),
            public_path: Self::normalize_path(public_path)
        }
    }

    pub fn private(&self) -> String {
        self.private_path.clone()
    }

    pub fn public(&self) -> String {
        self.public_path.clone()
    }

    pub fn database(&self) -> String {
        PathBuf::from(&self.private_path).join("surrealdb.db").to_string_lossy().to_string()
    }

    pub fn thumbnails(&self, path: String) -> String {
        PathBuf::from(&self.private_path).join(self.thumbnails_relative(path)).to_string_lossy().to_string()
    }

    pub fn thumbnails_relative(&self, path: String) -> String {
        PathBuf::from("thumbnails").join(Self::normalize_path(path)).to_string_lossy().to_string()
    }

    pub fn resources(&self, session_id: u64, path: String) -> String {
        PathBuf::from(&self.public_path).join(self.resources_relative(session_id, path)).to_string_lossy().to_string()
    }

    pub fn resources_relative(&self, session_id: u64, path: String) -> String {
        PathBuf::from(format!("session-{session_id}")).join(Self::normalize_path(path)).to_string_lossy().to_string()
    }

    pub fn to_absolute_path(&self, path: &LocalResourcePath) -> LocalResourcePath {
        match path {
            LocalResourcePath::RelativePath { path, is_private } => {
                LocalResourcePath::AbsolutePath(if *is_private {
                    PathBuf::from(&self.private_path).join(path).to_string_lossy().to_string()
                } else {
                    PathBuf::from(&self.public_path).join(path).to_string_lossy().to_string()
                })
            }
            LocalResourcePath::AbsolutePath(path) => LocalResourcePath::AbsolutePath(path.clone()),
            LocalResourcePath::PlatformIdentifier(path) => LocalResourcePath::PlatformIdentifier(path.clone())
        }
    }

    pub fn to_relative_path(&self, path: &LocalResourcePath) -> LocalResourcePath {
        match path {
            LocalResourcePath::AbsolutePath(path) => {
                let is_private = path.starts_with(&self.private_path);
                let relative = self.to_relative(path.clone());
                LocalResourcePath::RelativePath { path: relative, is_private }
            }
            _ => path.clone()
        }
    }

    pub fn to_relative(&self, path: String) -> String {
        let path_buf = PathBuf::from(&path);
        let private_path = PathBuf::from(&self.private_path);
        let public_path = PathBuf::from(&self.public_path);

        if let Ok(relative) = path_buf.strip_prefix(&private_path) {
            relative.to_string_lossy().to_string()
        } else if let Ok(relative) = path_buf.strip_prefix(&public_path) {
            relative.to_string_lossy().to_string()
        } else {
            path
        }
    }

    // Helper method to normalize paths once during construction
    fn normalize_path(path: String) -> String {
        PathBuf::from(path).as_os_str().to_string_lossy().to_string()
    }
}
