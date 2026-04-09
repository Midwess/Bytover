use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::SystemTime;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileEntry {
    pub is_dir: bool,
    pub modified_at: SystemTime,
    pub size: u64,
    pub path: PathBuf
}

impl FileEntry {
    pub fn name(&self) -> String {
        self.path.file_name().unwrap().to_string_lossy().into_owned()
    }

    pub fn relative_path(&self, base_path: &PathBuf) -> Option<PathBuf> {
        self.path.strip_prefix(base_path).ok().map(|it| it.to_path_buf())
    }
}
