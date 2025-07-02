use crate::app::file_system::file::LocalResourcePath;
use std::path::PathBuf;

#[async_trait::async_trait]
pub trait PathResolver: Send + Sync {
    async fn get_absolute_path(&self, path: LocalResourcePath) -> String;

    async fn get_local_resource_path(&self, absolute_path: String) -> LocalResourcePath;

    async fn get_thumbnail_dir_path(&self) -> String;

    async fn get_session_dir_path(&self, session_id: u64) -> String;

    async fn get_system_dir_path(&self) -> String;

    async fn get_db_path(&self) -> String;

    async fn get_thumbnail_file_path(&self, resource_id: u64) -> String {
        let path = PathBuf::from(self.get_thumbnail_dir_path().await).join(format!("{resource_id}.png"));
        path.to_string_lossy().to_string()
    }
}
