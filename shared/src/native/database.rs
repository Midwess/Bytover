use core_services::{db::repository::abstraction::local_repository::LocalSurrealDbRepository, utils::pool::{allocator::PoolAllocator, request::PoolRequest}};

use crate::{app::operations::database::{DatabaseOperation, DatabaseOperationOutput, TokenOperation, TokenOperationOutput}, entities::token::Token, persistence::token::TokenRepository};

pub struct NativeDatabase {
    pub token_repository: TokenRepository
}

impl NativeDatabase {
    pub async fn handle(&self, effect: DatabaseOperation) -> DatabaseOperationOutput {
        match effect {
            DatabaseOperation::Token(TokenOperation::Write(token)) => {
                let result = self.token_repository.create(token).await;
                DatabaseOperationOutput::Token(TokenOperationOutput::Write())
            },
            DatabaseOperation::Token(TokenOperation::Latest()) => {
                match self.token_repository.get_latest_token().await {
                    Ok(token) => {
                        DatabaseOperationOutput::Token(TokenOperationOutput::Latest { token: token, error: None })
                    },
                    Err(error) => {
                        DatabaseOperationOutput::Token(TokenOperationOutput::Latest { token: None, error: Some(error.to_string()) })
                    }
                }
            }
            _ => panic!("Native database doesn't support this effect {:?}", effect)
        }
    }
}   