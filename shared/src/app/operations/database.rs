use std::future::Future;

use crux_core::{capability::Operation, Command};
use serde::{Deserialize, Serialize};
use uniffi::Enum;

use crate::{app::AppRequestBuilder, entities::token::Token};

use super::{CoreOperation, CoreOperationOutput};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Enum)]
pub enum DatabaseOperation {
    Token(TokenOperation)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Enum)]
pub enum TokenOperation {
    Write(Token),
    Latest()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Enum)]
pub enum TokenOperationOutput {
    Write(),
    Latest {
        token: Option<Token>,
        error: Option<String>
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Enum)]
pub enum DatabaseOperationOutput {
    Token(TokenOperationOutput)
}

impl Operation for DatabaseOperation {
    type Output = DatabaseOperationOutput;
}

impl TokenOperation{
    pub fn save(token: Token) -> AppRequestBuilder<impl Future<Output = ()>> {
        Command::request_from_shell(CoreOperation::Database(DatabaseOperation::Token(TokenOperation::Write(token))))
        .map(|it| {
            match it {
                CoreOperationOutput::Database(DatabaseOperationOutput::Token(TokenOperationOutput::Write())) => {
                    ()
                },
                _ => panic!("Invalid output")
            }
        })
    }

    pub fn latest() -> AppRequestBuilder<impl Future<Output = Result<Option<Token>, String>>> {
        Command::request_from_shell(CoreOperation::Database(DatabaseOperation::Token(TokenOperation::Latest())))
        .map(|it| {
            match it {
                CoreOperationOutput::Database(DatabaseOperationOutput::Token(TokenOperationOutput::Latest { token, error })) => {
                    if let Some(error) = error {
                        Err(error)
                    } else {
                        Ok(token)
                    }

                },
                _ => panic!("Invalid output")
            }
        })
    }
}
