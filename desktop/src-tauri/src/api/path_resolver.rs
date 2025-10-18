use std::path::PathBuf;
use tokio::fs::create_dir_all;
use shared::entities::local_resource::LocalResourcePath;
use shared::repository::path_resolver::PathResolver;

pub struct PathResolverImpl {
    private_dir_path: PathBuf,
    user_dir_path: PathBuf,
    thumbnails_dir_path: PathBuf,
}

impl PathResolverImpl {
    pub async fn new(workdir_path: PathBuf) -> Self {
        let private_dir_path = workdir_path.join("private");
        let user_dir_path = workdir_path.join("user");
        let thumbnails_dir_path = private_dir_path.join("thumbnails");

        if !thumbnails_dir_path.exists() {
            let _ = create_dir_all(&thumbnails_dir_path).await;
        }

        if !private_dir_path.exists() {
            let _ = create_dir_all(&private_dir_path).await;
        }

        if !user_dir_path.exists() {
            let _ = create_dir_all(&user_dir_path).await;
        }

        Self {
            private_dir_path,
            user_dir_path,
            thumbnails_dir_path
        }
    }
}

#[async_trait::async_trait]
impl PathResolver for PathResolverImpl {
    async fn get_absolute_path(&self, path: LocalResourcePath) -> String {
        match path {
            LocalResourcePath::AbsolutePath(str) => str,
            LocalResourcePath::RelativePath { path, is_private} => match is_private {
                true => self.private_dir_path.join(path).to_str().unwrap().to_string(),
                false => self.user_dir_path.join(path).to_str().unwrap().to_string()
            }
            LocalResourcePath::PlatformIdentifier(str) => str
        }
    }

    async fn get_local_resource_path(&self, absolute_path: String) -> LocalResourcePath {
        LocalResourcePath::AbsolutePath(absolute_path)
    }

    async fn get_thumbnail_dir_path(&self) -> String {
        self.thumbnails_dir_path.to_str().unwrap().to_string()
    }

    async fn get_session_dir_path(&self, session_id: u64) -> String {
        let session_dir_path = self.private_dir_path.join(format!("session_{}", session_id));
        if !session_dir_path.exists() {
            let _ = create_dir_all(&session_dir_path).await;
        }

        session_dir_path.to_str().unwrap().to_string()
    }

    async fn get_system_dir_path(&self) -> String {
        self.private_dir_path.to_str().unwrap().to_string()
    }

    async fn get_db_path(&self) -> String {
        let system_dir = self.get_system_dir_path().await;
        let db_path = PathBuf::from(system_dir).join("database.redb");
        db_path.to_str().unwrap().to_string()
    }
}
