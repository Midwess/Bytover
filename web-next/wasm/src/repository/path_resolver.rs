use crate::ShellRuntime;
use serde::{Deserialize, Serialize};
use shared::app::file_system::file::LocalResourcePath;
use shared::app::repository::path_resolver::PathResolver;
use std::sync::Arc;
use crate::executor::message_to_shell::{MessageToShell, MessageToShellResponse};

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub enum PathResolverMessage {
    GetAbsolutePath { path: LocalResourcePath },
    GetLocalResourcePath { absolute_path: String },
    GetThumbnailDirPath,
    GetSessionDirPath { session_id: u64 },
    GetSystemDirPath
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub enum PathResolverResponseMessage {
    GetAbsolutePath { absolute_path: String },
    GetLocalResourcePath { path: LocalResourcePath },
    GetThumbnailDirPath { path: String },
    GetSessionDirPath { path: String },
    GetSystemDirPath { path: String }
}

pub struct PathResolverImpl {
    pub shell: Arc<ShellRuntime>
}

#[async_trait::async_trait(?Send)]
impl PathResolver for PathResolverImpl {
    async fn get_absolute_path(&self, path: LocalResourcePath) -> String {
        let MessageToShellResponse::PathResolverResponse(PathResolverResponseMessage::GetAbsolutePath { absolute_path }) = self
            .shell
            .request(MessageToShell::PathResolver(PathResolverMessage::GetAbsolutePath { path }))
            .await
        else {
            panic!("Failed to get absolute path");
        };

        absolute_path
    }

    async fn get_local_resource_path(&self, absolute_path: String) -> LocalResourcePath {
        let MessageToShellResponse::PathResolverResponse(PathResolverResponseMessage::GetLocalResourcePath { path }) = self
            .shell
            .request(MessageToShell::PathResolver(PathResolverMessage::GetLocalResourcePath {
                absolute_path
            }))
            .await
        else {
            panic!("Failed to get local resource path");
        };

        path
    }

    async fn get_thumbnail_dir_path(&self) -> String {
        let MessageToShellResponse::PathResolverResponse(PathResolverResponseMessage::GetThumbnailDirPath { path }) =
            self.shell.request(MessageToShell::PathResolver(PathResolverMessage::GetThumbnailDirPath)).await
        else {
            panic!("Failed to get thumbnail dir path");
        };

        path
    }

    async fn get_session_dir_path(&self, session_id: u64) -> String {
        let MessageToShellResponse::PathResolverResponse(PathResolverResponseMessage::GetSessionDirPath { path }) = self
            .shell
            .request(MessageToShell::PathResolver(PathResolverMessage::GetSessionDirPath {
                session_id
            }))
            .await
        else {
            panic!("Failed to get session dir path");
        };

        path
    }

    async fn get_system_dir_path(&self) -> String {
        let MessageToShellResponse::PathResolverResponse(PathResolverResponseMessage::GetSystemDirPath { path }) =
            self.shell.request(MessageToShell::PathResolver(PathResolverMessage::GetSystemDirPath)).await
        else {
            panic!("Failed to get system dir path");
        };

        path
    }

    async fn get_db_path(&self) -> String {
        panic!("Db in wasm is index db");
    }
}
