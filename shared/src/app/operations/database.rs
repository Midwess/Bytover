use std::future::Future;

use crux_core::capability::Operation;
use crux_core::Command;
use serde::{Deserialize, Serialize};
use uniffi::Enum;

use crate::app::AppRequestBuilder;
use crate::entities::session::Session;
use crate::entities::token::Token;
use crate::entities::user::User;

use super::{CoreOperation, CoreOperationOutput};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Enum)]
pub enum DatabaseOperation {
    Session(SessionOperation),
    User(UserDatabaseOperation)
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
    User(UserDatabaseOperationOutput)
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
            _ => panic!("Invalid output")
        })
    }

    pub fn save_user(user: User) -> AppRequestBuilder<impl Future<Output = ()>> {
        Command::request_from_shell(CoreOperation::Database(DatabaseOperation::Session(
            SessionOperation::WriteUser(user)
        )))
        .map(|it| match it {
            CoreOperationOutput::Database(DatabaseOperationOutput::Session(SessionOperationOutput::WriteUser())) => (),
            _ => panic!("Invalid output")
        })
    }

    pub fn get_session() -> AppRequestBuilder<impl Future<Output = Option<Session>>> {
        Command::request_from_shell(CoreOperation::Database(DatabaseOperation::Session(SessionOperation::Get()))).map(
            |it| match it {
                CoreOperationOutput::Database(DatabaseOperationOutput::Session(SessionOperationOutput::Get(
                    session
                ))) => session,
                _ => panic!("Invalid output")
            }
        )
    }
}
