use std::future::Future;

use crux_core::capability::Operation;
use crux_core::Command;
use serde::{Deserialize, Serialize};
use uniffi::Enum;

use crate::app::file_system::file::{LocalResource, LocalResourcePath};
use crate::app::file_system::workdir::WorkDir;
use crate::app::transfer::session::{TransferProgress, TransferSession};
use crate::app::AppRequestBuilder;
use crate::entities::session::Session;
use crate::entities::token::Token;
use crate::entities::user::User;
use crate::persistence::transfer_session::TransferSessionId;

use super::{CoreOperation, CoreOperationOutput};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Enum)]
pub enum DatabaseOperation {
    GenId(),
    Session(SessionOperation),
    User(UserDatabaseOperation),
    LocalResource(LocalResourceDatabaseOperation),
    TransferSession(TransferSessionOperation)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Enum)]
pub enum LocalResourceDatabaseOperation {
    Add(Vec<LocalResource>),
    Remove(u64),
    Find(LocalResourcePath),
    FindAll,
    Update(LocalResource)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Enum)]
pub enum LocalResourceDatabaseOperationOutput {
    Add(Vec<LocalResource>),
    Remove(Option<LocalResource>),
    Find(Option<LocalResource>),
    FindAll(Vec<LocalResource>),
    Update(Option<LocalResource>)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Enum)]
pub enum UserDatabaseOperation {
    Save(User)
}

impl Operation for UserDatabaseOperation {
    type Output = UserDatabaseOperationOutput;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Enum)]
pub enum UserDatabaseOperationOutput {
    Save()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Enum)]
pub enum SessionOperation {
    WriteToken(Token),
    WriteUser(User),
    Get()
}

impl Operation for SessionOperation {
    type Output = SessionOperationOutput;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Enum)]
pub enum SessionOperationOutput {
    WriteToken(),
    WriteUser(),
    Get(Option<Session>)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Enum)]
pub enum TransferSessionOperation {
    Save(TransferSession),
    UpdateProgresses(u64, Vec<TransferProgress>),
    Remove(u64),
    GetAll(TransferSessionId),
    UpdateResource { session_id: u64, resource: LocalResource }
}

impl Operation for TransferSessionOperation {
    type Output = TransferSessionOperationOutput;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Enum)]
pub enum TransferSessionOperationOutput {
    Save(Option<TransferSession>),
    UpdateProgresses(Option<TransferSession>),
    Remove(Option<TransferSession>),
    GetAll(Vec<TransferSession>),
    UpdateResource(Option<TransferSession>)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Enum)]
pub enum DatabaseOperationOutput {
    Session(SessionOperationOutput),
    User(UserDatabaseOperationOutput),
    LocalResource(LocalResourceDatabaseOperationOutput),
    GenId(u64),
    TransferSession(TransferSessionOperationOutput)
}

impl Operation for DatabaseOperation {
    type Output = DatabaseOperationOutput;
}

impl DatabaseOperation {
    pub fn gen_id() -> AppRequestBuilder<impl Future<Output = u64>> {
        Command::request_from_shell(CoreOperation::Database(DatabaseOperation::GenId())).map(|it| match it {
            CoreOperationOutput::Database(DatabaseOperationOutput::GenId(id)) => id,
            _ => panic!("Invalid output expected GenId got {it:?}")
        })
    }
}

impl SessionOperation {
    pub fn save_token(token: Token) -> AppRequestBuilder<impl Future<Output = ()>> {
        Command::request_from_shell(CoreOperation::Database(DatabaseOperation::Session(
            SessionOperation::WriteToken(token)
        )))
        .map(|it| match it {
            CoreOperationOutput::Database(DatabaseOperationOutput::Session(SessionOperationOutput::WriteToken())) => {}
            _ => panic!("Invalid output expected WriteToken got {it:?}")
        })
    }

    pub fn save_user(user: User) -> AppRequestBuilder<impl Future<Output = ()>> {
        Command::request_from_shell(CoreOperation::Database(DatabaseOperation::Session(
            SessionOperation::WriteUser(user)
        )))
        .map(|it| match it {
            CoreOperationOutput::Database(DatabaseOperationOutput::Session(SessionOperationOutput::WriteUser())) => (),
            _ => panic!("Invalid output expected WriteUser got {it:?}")
        })
    }

    pub fn get_session() -> AppRequestBuilder<impl Future<Output = Option<Session>>> {
        Command::request_from_shell(CoreOperation::Database(DatabaseOperation::Session(SessionOperation::Get()))).map(|it| match it {
            CoreOperationOutput::Database(DatabaseOperationOutput::Session(SessionOperationOutput::Get(session))) => session,
            _ => panic!("Invalid output expected Get got {it:?}")
        })
    }
}

impl LocalResourceDatabaseOperation {
    pub fn add(resources: Vec<LocalResource>) -> AppRequestBuilder<impl Future<Output = Vec<LocalResource>>> {
        Command::request_from_shell(CoreOperation::Database(DatabaseOperation::LocalResource(
            LocalResourceDatabaseOperation::Add(resources)
        )))
        .map(|it| match it {
            CoreOperationOutput::Database(DatabaseOperationOutput::LocalResource(LocalResourceDatabaseOperationOutput::Add(
                resources
            ))) => resources,
            _ => panic!("Invalid output expected Add got {it:?}")
        })
    }

    pub fn remove(id: u64) -> AppRequestBuilder<impl Future<Output = Option<LocalResource>>> {
        Command::request_from_shell(CoreOperation::Database(DatabaseOperation::LocalResource(
            LocalResourceDatabaseOperation::Remove(id)
        )))
        .map(|it| match it {
            CoreOperationOutput::Database(DatabaseOperationOutput::LocalResource(LocalResourceDatabaseOperationOutput::Remove(
                resource
            ))) => resource,
            _ => panic!("Invalid output expected Remove got {it:?}")
        })
    }

    pub fn find(path: LocalResourcePath) -> AppRequestBuilder<impl Future<Output = Option<LocalResource>>> {
        Command::request_from_shell(CoreOperation::Database(DatabaseOperation::LocalResource(
            LocalResourceDatabaseOperation::Find(path)
        )))
        .map(|it| match it {
            CoreOperationOutput::Database(DatabaseOperationOutput::LocalResource(LocalResourceDatabaseOperationOutput::Find(
                resource
            ))) => resource,
            _ => panic!("Invalid output expected Find got {it:?}")
        })
    }

    pub fn find_all() -> AppRequestBuilder<impl Future<Output = Vec<LocalResource>>> {
        Command::request_from_shell(CoreOperation::Database(DatabaseOperation::LocalResource(
            LocalResourceDatabaseOperation::FindAll
        )))
        .map(|it| match it {
            CoreOperationOutput::Database(DatabaseOperationOutput::LocalResource(LocalResourceDatabaseOperationOutput::FindAll(
                resources
            ))) => resources,
            _ => panic!("Invalid output expected FindAll got {it:?}")
        })
    }

    pub fn update(resource: LocalResource) -> AppRequestBuilder<impl Future<Output = Option<LocalResource>>> {
        Command::request_from_shell(CoreOperation::Database(DatabaseOperation::LocalResource(
            LocalResourceDatabaseOperation::Update(resource)
        )))
        .map(|it| match it {
            CoreOperationOutput::Database(DatabaseOperationOutput::LocalResource(LocalResourceDatabaseOperationOutput::Update(
                resource
            ))) => resource,
            _ => panic!("Invalid output expected Update got {it:?}")
        })
    }
}

impl TransferSessionOperation {
    pub fn save(mut session: TransferSession, workdir: &WorkDir) -> AppRequestBuilder<impl Future<Output = Option<TransferSession>>> {
        session.resources.iter_mut().for_each(|resource| {
            resource.path = workdir.to_relative_path(&resource.path);
            resource.thumbnail_path = resource.thumbnail_path.as_ref().map(|path| workdir.to_relative_path(path));
        });

        Command::request_from_shell(CoreOperation::Database(DatabaseOperation::TransferSession(
            TransferSessionOperation::Save(session)
        )))
        .map(|it| match it {
            CoreOperationOutput::Database(DatabaseOperationOutput::TransferSession(TransferSessionOperationOutput::Save(session))) => {
                session
            }
            _ => panic!("Invalid output expected Save got {it:?}")
        })
    }

    pub fn update_progresses(
        order_id: u64,
        progresses: Vec<TransferProgress>
    ) -> AppRequestBuilder<impl Future<Output = Option<TransferSession>>> {
        Command::request_from_shell(CoreOperation::Database(DatabaseOperation::TransferSession(
            TransferSessionOperation::UpdateProgresses(order_id, progresses)
        )))
        .map(|it| match it {
            CoreOperationOutput::Database(DatabaseOperationOutput::TransferSession(
                TransferSessionOperationOutput::UpdateProgresses(session)
            )) => session,
            _ => panic!("Invalid output expected UpdateProgresses got {it:?}")
        })
    }

    pub fn update_resource(
        session_id: u64,
        mut resource: LocalResource,
        workdir: &WorkDir
    ) -> AppRequestBuilder<impl Future<Output = Option<TransferSession>>> {
        resource.path = workdir.to_relative_path(&resource.path);
        resource.thumbnail_path = resource.thumbnail_path.as_ref().map(|path| workdir.to_relative_path(path));

        Command::request_from_shell(CoreOperation::Database(DatabaseOperation::TransferSession(
            TransferSessionOperation::UpdateResource { session_id, resource }
        )))
        .map(|it| match it {
            CoreOperationOutput::Database(DatabaseOperationOutput::TransferSession(
                TransferSessionOperationOutput::UpdateResource(session)
            )) => session,
            _ => panic!("Invalid output expected UpdateResource got {it:?}")
        })
    }

    pub fn get_all(id: TransferSessionId, workdir: WorkDir) -> AppRequestBuilder<impl Future<Output = Vec<TransferSession>>> {
        Command::request_from_shell(CoreOperation::Database(DatabaseOperation::TransferSession(
            TransferSessionOperation::GetAll(id)
        )))
        .map(move |it| match it {
            CoreOperationOutput::Database(DatabaseOperationOutput::TransferSession(TransferSessionOperationOutput::GetAll(
                mut sessions
            ))) => {
                sessions.iter_mut().for_each(|session| {
                    session.resources.iter_mut().for_each(|resource| {
                        resource.path = workdir.to_absolute_path(&resource.path);
                        resource.thumbnail_path = resource.thumbnail_path.as_ref().map(|path| workdir.to_absolute_path(path));
                    });
                });

                sessions
            }
            _ => panic!("Invalid output expected GetAll got {it:?}")
        })
    }

    pub fn remove(id: u64) -> AppRequestBuilder<impl Future<Output = Option<TransferSession>>> {
        Command::request_from_shell(CoreOperation::Database(DatabaseOperation::TransferSession(
            TransferSessionOperation::Remove(id)
        )))
        .map(|it| match it {
            CoreOperationOutput::Database(DatabaseOperationOutput::TransferSession(TransferSessionOperationOutput::Remove(
                session
            ))) => session,
            _ => panic!("Invalid output expected Remove got {it:?}")
        })
    }
}
