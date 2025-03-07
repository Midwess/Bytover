use std::future::Future;

use crux_core::capability::Operation;
use crux_core::Command;
use serde::{Deserialize, Serialize};
use uniffi::Enum;

use crate::app::file_system::file::LocalResourcePath;
use crate::app::{file_system::file::LocalResource, AppRequestBuilder};
use crate::entities::session::Session;
use crate::entities::token::Token;
use crate::entities::user::User;

use super::{CoreOperation, CoreOperationOutput};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Enum)]
pub enum DatabaseOperation {
    Session(SessionOperation),
    User(UserDatabaseOperation),
    LocalResource(LocalResourceDatabaseOperation),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Enum)]
pub enum LocalResourceDatabaseOperation {
    Add(Vec<LocalResource>),
    Remove(u64),
    Find(LocalResourcePath),
    FindAll
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Enum)]
pub enum LocalResourceDatabaseOperationOutput {
    Add(Vec<LocalResource>),
    Remove(Option<LocalResource>),
    Find(Option<LocalResource>),
    FindAll(Vec<LocalResource>)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Enum)]
pub enum UserDatabaseOperation {
    Save(User)
}

impl Operation for UserDatabaseOperation {
    type Output = UserDatabaseOperationOutput;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Enum)]
pub enum UserDatabaseOperationOutput {
    Save()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Enum)]
pub enum SessionOperation {
    WriteToken(Token),
    WriteUser(User),
    Get()
}

impl Operation for SessionOperation {
    type Output = SessionOperationOutput;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Enum)]
pub enum SessionOperationOutput {
    WriteToken(),
    WriteUser(),
    Get(Option<Session>)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Enum)]
pub enum DatabaseOperationOutput {
    Session(SessionOperationOutput),
    User(UserDatabaseOperationOutput),
    LocalResource(LocalResourceDatabaseOperationOutput)
}

impl Operation for DatabaseOperation {
    type Output = DatabaseOperationOutput;
}

impl SessionOperation {
    pub fn save_token(token: Token) -> AppRequestBuilder<impl Future<Output = ()>> {
        Command::request_from_shell(CoreOperation::Database(DatabaseOperation::Session(
            SessionOperation::WriteToken(token)
        )))
        .map(|it| match it {
            CoreOperationOutput::Database(DatabaseOperationOutput::Session(SessionOperationOutput::WriteToken())) => {}
            _ => panic!("Invalid output expected WriteToken got {:?}", it)
        })
    }

    pub fn save_user(user: User) -> AppRequestBuilder<impl Future<Output = ()>> {
        Command::request_from_shell(CoreOperation::Database(DatabaseOperation::Session(
            SessionOperation::WriteUser(user)
        )))
        .map(|it| match it {
            CoreOperationOutput::Database(DatabaseOperationOutput::Session(SessionOperationOutput::WriteUser())) => (),
            _ => panic!("Invalid output expected WriteUser got {:?}", it)
        })
    }

    pub fn get_session() -> AppRequestBuilder<impl Future<Output = Option<Session>>> {
        Command::request_from_shell(CoreOperation::Database(DatabaseOperation::Session(SessionOperation::Get()))).map(
            |it| match it {
                CoreOperationOutput::Database(DatabaseOperationOutput::Session(SessionOperationOutput::Get(
                    session
                ))) => session,
                _ => panic!("Invalid output expected Get got {:?}", it)
            }
        )
    }
}

impl LocalResourceDatabaseOperation {
    pub fn add(resources: Vec<LocalResource>) -> AppRequestBuilder<impl Future<Output = Vec<LocalResource>>> {
        Command::request_from_shell(CoreOperation::Database(DatabaseOperation::LocalResource(
            LocalResourceDatabaseOperation::Add(resources)
        )))
        .map(|it| match it {
            CoreOperationOutput::Database(DatabaseOperationOutput::LocalResource(LocalResourceDatabaseOperationOutput::Add(resources))) => resources,
            _ => panic!("Invalid output expected Add got {:?}", it)
        })
    }

    pub fn remove(id: u64) -> AppRequestBuilder<impl Future<Output = Option<LocalResource>>> {
        Command::request_from_shell(CoreOperation::Database(DatabaseOperation::LocalResource(
            LocalResourceDatabaseOperation::Remove(id)
        )))
        .map(|it| match it {
            CoreOperationOutput::Database(DatabaseOperationOutput::LocalResource(LocalResourceDatabaseOperationOutput::Remove(resource))) => resource,
            _ => panic!("Invalid output expected Remove got {:?}", it)
        })
    }

    pub fn find(path: LocalResourcePath) -> AppRequestBuilder<impl Future<Output = Option<LocalResource>>> {
        Command::request_from_shell(CoreOperation::Database(DatabaseOperation::LocalResource(
            LocalResourceDatabaseOperation::Find(path)
        )))
        .map(|it| match it {
            CoreOperationOutput::Database(DatabaseOperationOutput::LocalResource(LocalResourceDatabaseOperationOutput::Find(resource))) => resource,
            _ => panic!("Invalid output expected Find got {:?}", it)
        })
    }

    pub fn find_all() -> AppRequestBuilder<impl Future<Output = Vec<LocalResource>>> {
        Command::request_from_shell(CoreOperation::Database(DatabaseOperation::LocalResource(
            LocalResourceDatabaseOperation::FindAll
        )))
        .map(|it| match it {
            CoreOperationOutput::Database(DatabaseOperationOutput::LocalResource(LocalResourceDatabaseOperationOutput::FindAll(resources))) => resources,
            _ => panic!("Invalid output expected FindAll got {:?}", it)
        })
    }
}