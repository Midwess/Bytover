use std::future::Future;

use crux_core::{capability::Operation, Command};
use serde::{Deserialize, Serialize};
use uniffi::Enum;

use crate::{app::AppRequestBuilder, entities::token::Token};

use super::{CoreOperation, CoreOperationOutput};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Enum)]
pub enum DatabaseOperation {
    SaveToken(Token)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Enum)]
pub enum DatabaseOperationOutput {
    SaveToken(),
}

impl Operation for DatabaseOperation {
    type Output = DatabaseOperationOutput;
}

impl DatabaseOperation {
    pub fn save_token(token: Token) -> AppRequestBuilder<impl Future<Output = ()>> {
        Command::request_from_shell(CoreOperation::Database(DatabaseOperation::SaveToken(token)))
        .map(|it| {
            match it {
                CoreOperationOutput::Database(DatabaseOperationOutput::SaveToken()) => {
                    ()
                }
                _ => panic!("Failed to save token")
            }
        })
    }
}