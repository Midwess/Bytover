use std::collections::HashMap;
use std::future::Future;

use crux_core::capability::Operation;
use serde::{Deserialize, Serialize};

use crate::app::core::command::AppCommand;
use crate::app::AppRequestBuilder;
use crate::entities::local_resource::{LocalResource, LocalResourcePath, ResourceType};
use crate::entities::session::Session;
use crate::entities::token::Token;
use crate::entities::transfer_session::{TransferProgress, TransferSession};
use crate::entities::user::User;
use crate::errors::CoreError;
use crate::repository::transfer_session::TransferSessionId;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PersistentOperation {
    Session(SessionPersistentOperation),
    User(UserPersistentOperation),
    LocalResource(LocalResourcePersistentOperation),
    TransferSession(TransferSessionPersistentOperation)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LocalResourcePersistentOperation {
    Add(Vec<LocalResource>),
    Remove(LocalResourcePath),
    FindAll,
    LoadOnDisk(LocalResourcePath),
    GetResourceType { path: LocalResourcePath },
    AddThumbnail { png_bytes: Vec<u8>, resource_id: u64 }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum UserPersistentOperation {
    Save(User)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SessionPersistentOperation {
    WriteToken(Token),
    WriteUser(User),
    Remove,
    Get()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TransferSessionPersistentOperation {
    Save(TransferSession),
    Clear,
    UpdateProgresses(u64, Vec<TransferProgress>),
    Remove(TransferSessionId),
    GetAllReceivedSessions(),
    UpdateResource {
        session_id: TransferSessionId,
        resource: LocalResource
    },
    GenerateResourcePath {
        session_id: u64,
        resource_names: HashMap<u64, String>
    },
    GenerateThumbnailPath {
        session_id: Option<u64>,
        resource_ids: Vec<u64>
    }
}

impl Operation for PersistentOperation {
    type Output = ();
}

impl SessionPersistentOperation {
    pub fn save_token(token: Token) -> AppRequestBuilder<impl Future<Output = Result<(), CoreError>>> {
        AppCommand::request_from_shell(PersistentOperation::Session(SessionPersistentOperation::WriteToken(token)))
            .map(|it| it.result())
    }

    pub fn save_user(user: User) -> AppRequestBuilder<impl Future<Output = Result<(), CoreError>>> {
        AppCommand::request_from_shell(PersistentOperation::Session(SessionPersistentOperation::WriteUser(user))).map(|it| it.result())
    }

    pub fn get_session() -> AppRequestBuilder<impl Future<Output = Result<Option<Session>, CoreError>>> {
        AppCommand::request_from_shell(PersistentOperation::Session(SessionPersistentOperation::Get())).map(|it| it.result_option())
    }

    pub fn remove_session() -> AppRequestBuilder<impl Future<Output = Result<(), CoreError>>> {
        AppCommand::request_from_shell(PersistentOperation::Session(SessionPersistentOperation::Remove)).map(|it| it.result())
    }
}

impl LocalResourcePersistentOperation {
    pub fn add(resources: Vec<LocalResource>) -> AppRequestBuilder<impl Future<Output = Result<Vec<LocalResource>, CoreError>>> {
        AppCommand::request_from_shell(PersistentOperation::LocalResource(LocalResourcePersistentOperation::Add(
            resources
        )))
        .map(|it| it.result())
    }

    pub fn remove(path: LocalResourcePath) -> AppRequestBuilder<impl Future<Output = Result<bool, CoreError>>> {
        AppCommand::request_from_shell(PersistentOperation::LocalResource(LocalResourcePersistentOperation::Remove(
            path
        )))
        .map(|it| it.result())
    }

    pub fn find_all() -> AppRequestBuilder<impl Future<Output = Result<Vec<LocalResource>, CoreError>>> {
        AppCommand::request_from_shell(PersistentOperation::LocalResource(LocalResourcePersistentOperation::FindAll))
            .map(|it| it.result())
    }

    pub fn add_thumbnail(
        png_bytes: Vec<u8>,
        resource_id: u64
    ) -> AppRequestBuilder<impl Future<Output = Result<LocalResourcePath, CoreError>>> {
        AppCommand::request_from_shell(PersistentOperation::LocalResource(
            LocalResourcePersistentOperation::AddThumbnail { png_bytes, resource_id }
        ))
        .map(|it| it.result())
    }

    pub fn load_from_disk(
        path: LocalResourcePath
    ) -> AppRequestBuilder<impl Future<Output = Result<Option<LocalResource>, CoreError>>> {
        AppCommand::request_from_shell(PersistentOperation::LocalResource(
            LocalResourcePersistentOperation::LoadOnDisk(path)
        ))
        .map(|it| it.result_option())
    }

    pub fn get_resource_type(path: LocalResourcePath) -> AppRequestBuilder<impl Future<Output = Result<ResourceType, CoreError>>> {
        AppCommand::request_from_shell(PersistentOperation::LocalResource(
            LocalResourcePersistentOperation::GetResourceType { path }
        ))
        .map(|it| it.result())
    }
}

impl TransferSessionPersistentOperation {
    pub fn save(session: TransferSession) -> AppRequestBuilder<impl Future<Output = Result<TransferSession, CoreError>>> {
        AppCommand::request_from_shell(PersistentOperation::TransferSession(TransferSessionPersistentOperation::Save(
            session
        )))
        .map(|it| it.result())
    }

    pub fn update_progresses(
        order_id: u64,
        progresses: Vec<TransferProgress>
    ) -> AppRequestBuilder<impl Future<Output = Result<Option<TransferSession>, CoreError>>> {
        AppCommand::request_from_shell(PersistentOperation::TransferSession(
            TransferSessionPersistentOperation::UpdateProgresses(order_id, progresses)
        ))
        .map(|it| it.result_option())
    }

    pub fn update_resource(
        session_id: TransferSessionId,
        resource: LocalResource
    ) -> AppRequestBuilder<impl Future<Output = Result<Option<TransferSession>, CoreError>>> {
        AppCommand::request_from_shell(PersistentOperation::TransferSession(
            TransferSessionPersistentOperation::UpdateResource { session_id, resource }
        ))
        .map(|it| it.result_option())
    }

    pub fn get_all_received_sessions() -> AppRequestBuilder<impl Future<Output = Result<Vec<TransferSession>, CoreError>>> {
        AppCommand::request_from_shell(PersistentOperation::TransferSession(
            TransferSessionPersistentOperation::GetAllReceivedSessions()
        ))
        .map(|it| it.result())
    }

    pub fn remove(id: TransferSessionId) -> AppRequestBuilder<impl Future<Output = Result<bool, CoreError>>> {
        AppCommand::request_from_shell(PersistentOperation::TransferSession(
            TransferSessionPersistentOperation::Remove(id)
        ))
        .map(|it| it.result())
    }

    pub fn clear_all() -> AppRequestBuilder<impl Future<Output = Result<bool, CoreError>>> {
        AppCommand::request_from_shell(PersistentOperation::TransferSession(TransferSessionPersistentOperation::Clear))
            .map(|it| it.result())
    }

    pub fn generate_thumbnail_paths(
        session_id: Option<u64>,
        resource_ids: Vec<u64>
    ) -> AppRequestBuilder<impl Future<Output = Result<HashMap<u64, LocalResourcePath>, CoreError>>> {
        AppCommand::request_from_shell(PersistentOperation::TransferSession(
            TransferSessionPersistentOperation::GenerateThumbnailPath { session_id, resource_ids }
        ))
        .map(|it| it.result())
    }

    pub fn generate_resource_paths(
        id: u64,
        resource_names: HashMap<u64, String>
    ) -> AppRequestBuilder<impl Future<Output = Result<HashMap<u64, LocalResourcePath>, CoreError>>> {
        AppCommand::request_from_shell(PersistentOperation::TransferSession(
            TransferSessionPersistentOperation::GenerateResourcePath {
                session_id: id,
                resource_names
            }
        ))
        .map(|it| it.result())
    }
}
