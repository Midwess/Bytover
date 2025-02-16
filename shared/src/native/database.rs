use std::sync::Arc;

use core_services::{db::repository::abstraction::repository::LocalSurrealDbRepository, utils::pool::{allocator::PoolAllocator, request::PoolRequest}};
use surreal_derive_plus::surreal_quote;
use surrealdb::{engine::local::Db, Surreal};

use crate::{app::operations::database::{DatabaseOperation, DatabaseOperationOutput}, persistence::token::TokenRepository};

pub struct NativeDatabase {
    token_repository: TokenRepository
}

impl NativeDatabase {
    pub async fn handle(&self, effect: DatabaseOperation) -> DatabaseOperationOutput {
        match effect {
            DatabaseOperation::SaveToken(token) => {
                self.token_repository.create(token).await;
                DatabaseOperationOutput::SaveToken()
            }
            _ => panic!("Native database doesn't support this effect {:?}", effect)
        }
    }
}   