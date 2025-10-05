use std::future::Future;

use super::{CoreOperation, CoreOperationOutput};
use crate::app::AppRequestBuilder;
use crate::entities::local_resource::{LocalResource, LocalResourcePath, ResourceType};
use crate::entities::session::Session;
use crate::entities::token::Token;
use crate::entities::transfer_session::{TransferProgress, TransferSession, TransferType};
use crate::entities::user::User;
use crate::repository::transfer_session::TransferSessionId;
use crux_core::capability::Operation;
use crux_core::Command;
use derive_more::From;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, From)]
pub enum PersistentOperation {
    GenId(),
    Session(SessionPersistentOperation),
    User(UserPersistentOperation),
    LocalResource(LocalResourcePersistentOperation),
    TransferSession(TransferSessionPersistentOperation)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LocalResourcePersistentOperation {
    Add(Vec<LocalResource>),
    Remove(u64),
    Find(LocalResourcePath),
    FindAll,
    LoadOnDisk(LocalResourcePath),
    GetResourceType { path: LocalResourcePath },
    AddThumbnail { png_bytes: Vec<u8>, resource_id: u64 }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LocalResourcePersistentOperationOutput {
    Add(Vec<LocalResource>),
    AddThumbnail(LocalResourcePath),
    LoadOnDisk(Option<LocalResource>),
    Removed,
    GetResourceType(ResourceType),
    Find(Option<LocalResource>),
    FindAll(Vec<LocalResource>)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum UserPersistentOperation {
    Save(User)
}

impl Operation for UserPersistentOperation {
    type Output = UserPersistentOperationOutput;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum UserPersistentOperationOutput {
    Save()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SessionPersistentOperation {
    WriteToken(Token),
    WriteUser(User),
    Get()
}

impl Operation for SessionPersistentOperation {
    type Output = SessionPersistentOperationOutput;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SessionPersistentOperationOutput {
    WriteToken(),
    WriteUser(),
    Get(Option<Session>)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TransferSessionPersistentOperation {
    Save(TransferSession),
    UpdateProgresses(u64, Vec<TransferProgress>),
    Remove((u64, TransferType)),
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

impl Operation for TransferSessionPersistentOperation {
    type Output = TransferSessionOperationOutput;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TransferSessionOperationOutput {
    Save(Option<TransferSession>),
    UpdateProgresses(Option<TransferSession>),
    Removed(bool),
    GetAll(Vec<TransferSession>),
    UpdateResource(Option<TransferSession>),
    GenerateResourcePath(HashMap<u64, LocalResourcePath>),
    GenerateThumbnailPath(HashMap<u64, LocalResourcePath>)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, From)]
pub enum PersistentOperationOutput {
    Session(SessionPersistentOperationOutput),
    User(UserPersistentOperationOutput),
    LocalResource(LocalResourcePersistentOperationOutput),
    GenId(u64),
    TransferSession(TransferSessionOperationOutput),
    Error(String)
}

impl Operation for PersistentOperation {
    type Output = PersistentOperationOutput;
}

impl PersistentOperation {
    pub fn gen_id() -> AppRequestBuilder<impl Future<Output = u64>> {
        Command::request_from_shell(CoreOperation::Persistent(PersistentOperation::GenId())).map(|it| match it {
            CoreOperationOutput::Persistent(PersistentOperationOutput::GenId(id)) => id,
            _ => panic!("Invalid output expected GenId got {it:?}")
        })
    }
}

impl SessionPersistentOperation {
    pub fn save_token(token: Token) -> AppRequestBuilder<impl Future<Output = ()>> {
        Command::request_from_shell(CoreOperation::Persistent(PersistentOperation::Session(
            SessionPersistentOperation::WriteToken(token)
        )))
        .map(|it| match it {
            CoreOperationOutput::Persistent(PersistentOperationOutput::Session(SessionPersistentOperationOutput::WriteToken())) => {}
            _ => panic!("Invalid output expected WriteToken got {it:?}")
        })
    }

    pub fn save_user(user: User) -> AppRequestBuilder<impl Future<Output = ()>> {
        Command::request_from_shell(CoreOperation::Persistent(PersistentOperation::Session(
            SessionPersistentOperation::WriteUser(user)
        )))
        .map(|it| match it {
            CoreOperationOutput::Persistent(PersistentOperationOutput::Session(SessionPersistentOperationOutput::WriteUser())) => (),
            _ => panic!("Invalid output expected WriteUser got {it:?}")
        })
    }

    pub fn get_session() -> AppRequestBuilder<impl Future<Output = Option<Session>>> {
        Command::request_from_shell(CoreOperation::Persistent(PersistentOperation::Session(
            SessionPersistentOperation::Get()
        )))
        .map(|it| match it {
            CoreOperationOutput::Persistent(PersistentOperationOutput::Session(SessionPersistentOperationOutput::Get(session))) => {
                session
            }
            _ => panic!("Invalid output expected Get got {it:?}")
        })
    }
}

impl LocalResourcePersistentOperation {
    pub fn add(resources: Vec<LocalResource>) -> AppRequestBuilder<impl Future<Output = Vec<LocalResource>>> {
        Command::request_from_shell(CoreOperation::Persistent(PersistentOperation::LocalResource(
            LocalResourcePersistentOperation::Add(resources)
        )))
        .map(|it| match it {
            CoreOperationOutput::Persistent(PersistentOperationOutput::LocalResource(LocalResourcePersistentOperationOutput::Add(
                resources
            ))) => resources,
            _ => panic!("Invalid output expected Add got {it:?}")
        })
    }

    pub fn remove(id: u64) -> AppRequestBuilder<impl Future<Output = bool>> {
        Command::request_from_shell(CoreOperation::Persistent(PersistentOperation::LocalResource(
            LocalResourcePersistentOperation::Remove(id)
        )))
        .map(|it| match it {
            CoreOperationOutput::Persistent(PersistentOperationOutput::LocalResource(
                LocalResourcePersistentOperationOutput::Removed
            )) => true,
            _ => false
        })
    }

    pub fn find(path: LocalResourcePath) -> AppRequestBuilder<impl Future<Output = Option<LocalResource>>> {
        Command::request_from_shell(CoreOperation::Persistent(PersistentOperation::LocalResource(
            LocalResourcePersistentOperation::Find(path)
        )))
        .map(|it| match it {
            CoreOperationOutput::Persistent(PersistentOperationOutput::LocalResource(LocalResourcePersistentOperationOutput::Find(
                resource
            ))) => resource,
            _ => panic!("Invalid output expected Find got {it:?}")
        })
    }

    pub fn find_all() -> AppRequestBuilder<impl Future<Output = Vec<LocalResource>>> {
        Command::request_from_shell(CoreOperation::Persistent(PersistentOperation::LocalResource(
            LocalResourcePersistentOperation::FindAll
        )))
        .map(|it| match it {
            CoreOperationOutput::Persistent(PersistentOperationOutput::LocalResource(
                LocalResourcePersistentOperationOutput::FindAll(resources)
            )) => resources,
            _ => panic!("Invalid output expected FindAll got {it:?}")
        })
    }

    pub fn add_thumbnail(png_bytes: Vec<u8>, resource_id: u64) -> AppRequestBuilder<impl Future<Output = LocalResourcePath>> {
        Command::request_from_shell(CoreOperation::Persistent(PersistentOperation::LocalResource(
            LocalResourcePersistentOperation::AddThumbnail { png_bytes, resource_id }
        )))
        .map(|it| match it {
            CoreOperationOutput::Persistent(PersistentOperationOutput::LocalResource(
                LocalResourcePersistentOperationOutput::AddThumbnail(thumbnail_url)
            )) => thumbnail_url,
            _ => panic!("Invalid output expected AddThumbnail got {it:?}")
        })
    }

    pub fn load_from_disk(path: LocalResourcePath) -> AppRequestBuilder<impl Future<Output = Option<LocalResource>>> {
        Command::request_from_shell(CoreOperation::Persistent(PersistentOperation::LocalResource(
            LocalResourcePersistentOperation::LoadOnDisk(path)
        )))
        .map(|it| match it {
            CoreOperationOutput::Persistent(PersistentOperationOutput::LocalResource(
                LocalResourcePersistentOperationOutput::LoadOnDisk(resource)
            )) => resource,
            _ => panic!("Invalid output expected IsExistedOnDisk got {it:?}")
        })
    }

    pub fn get_resource_type(path: LocalResourcePath) -> AppRequestBuilder<impl Future<Output = ResourceType>> {
        Command::request_from_shell(CoreOperation::Persistent(PersistentOperation::LocalResource(
            LocalResourcePersistentOperation::GetResourceType { path }
        )))
        .map(|it| match it {
            CoreOperationOutput::Persistent(PersistentOperationOutput::LocalResource(
                LocalResourcePersistentOperationOutput::GetResourceType(resource_type)
            )) => resource_type,
            _ => panic!("Invalid output expected GetResourceType got {it:?}")
        })
    }
}

impl TransferSessionPersistentOperation {
    pub fn save(session: TransferSession) -> AppRequestBuilder<impl Future<Output = Option<TransferSession>>> {
        Command::request_from_shell(CoreOperation::Persistent(PersistentOperation::TransferSession(
            TransferSessionPersistentOperation::Save(session)
        )))
        .map(|it| match it {
            CoreOperationOutput::Persistent(PersistentOperationOutput::TransferSession(TransferSessionOperationOutput::Save(
                session
            ))) => session,
            _ => panic!("Invalid output expected Save got {it:?}")
        })
    }

    pub fn update_progresses(
        order_id: u64,
        progresses: Vec<TransferProgress>
    ) -> AppRequestBuilder<impl Future<Output = Option<TransferSession>>> {
        Command::request_from_shell(CoreOperation::Persistent(PersistentOperation::TransferSession(
            TransferSessionPersistentOperation::UpdateProgresses(order_id, progresses)
        )))
        .map(|it| match it {
            CoreOperationOutput::Persistent(PersistentOperationOutput::TransferSession(
                TransferSessionOperationOutput::UpdateProgresses(session)
            )) => session,
            _ => panic!("Invalid output expected UpdateProgresses got {it:?}")
        })
    }

    pub fn update_resource(
        session_id: TransferSessionId,
        resource: LocalResource
    ) -> AppRequestBuilder<impl Future<Output = Option<TransferSession>>> {
        Command::request_from_shell(CoreOperation::Persistent(PersistentOperation::TransferSession(
            TransferSessionPersistentOperation::UpdateResource { session_id, resource }
        )))
        .map(|it| match it {
            CoreOperationOutput::Persistent(PersistentOperationOutput::TransferSession(
                TransferSessionOperationOutput::UpdateResource(session)
            )) => session,
            _ => panic!("Invalid output expected UpdateResource got {it:?}")
        })
    }

    pub fn get_all_received_sessions() -> AppRequestBuilder<impl Future<Output = Vec<TransferSession>>> {
        Command::request_from_shell(CoreOperation::Persistent(PersistentOperation::TransferSession(
            TransferSessionPersistentOperation::GetAllReceivedSessions()
        )))
        .map(move |it| match it {
            CoreOperationOutput::Persistent(PersistentOperationOutput::TransferSession(TransferSessionOperationOutput::GetAll(
                sessions
            ))) => sessions,
            _ => panic!("Invalid output expected GetAll got {it:?}")
        })
    }

    pub fn remove(id: u64, transfer_type: TransferType) -> AppRequestBuilder<impl Future<Output = bool>> {
        Command::request_from_shell(CoreOperation::Persistent(PersistentOperation::TransferSession(
            TransferSessionPersistentOperation::Remove((id, transfer_type))
        )))
        .map(|it| match it {
            CoreOperationOutput::Persistent(PersistentOperationOutput::TransferSession(TransferSessionOperationOutput::Removed(
                is_removed
            ))) => is_removed,
            _ => panic!("Invalid output expected Remove got {it:?}")
        })
    }

    pub fn generate_thumbnail_paths(
        session_id: Option<u64>,
        resource_ids: Vec<u64>
    ) -> AppRequestBuilder<impl Future<Output = HashMap<u64, LocalResourcePath>>> {
        Command::request_from_shell(CoreOperation::Persistent(PersistentOperation::TransferSession(
            TransferSessionPersistentOperation::GenerateThumbnailPath { session_id, resource_ids }
        )))
        .map(|it| match it {
            CoreOperationOutput::Persistent(PersistentOperationOutput::TransferSession(
                TransferSessionOperationOutput::GenerateThumbnailPath(resource_paths)
            )) => resource_paths,
            _ => panic!("Invalid output expected GenerateResourcePath got {it:?}")
        })
    }

    pub fn generate_resource_paths(
        id: u64,
        resource_names: HashMap<u64, String>
    ) -> AppRequestBuilder<impl Future<Output = HashMap<u64, LocalResourcePath>>> {
        Command::request_from_shell(CoreOperation::Persistent(PersistentOperation::TransferSession(
            TransferSessionPersistentOperation::GenerateResourcePath {
                session_id: id,
                resource_names
            }
        )))
        .map(|it| match it {
            CoreOperationOutput::Persistent(PersistentOperationOutput::TransferSession(
                TransferSessionOperationOutput::GenerateResourcePath(resource_paths)
            )) => resource_paths,
            _ => panic!("Invalid output expected GenerateResourcePath got {it:?}")
        })
    }
}
