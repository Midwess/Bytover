use core_services::db::repository::abstraction::id::DbId;
use core_services::db::repository::abstraction::local_repository::LocalSurrealDbRepository;
use core_services::db::repository::abstraction::table::Table;
use core_services::utils::pool::reponse::PoolResponse;
use core_services::utils::pool::request::PoolRequest;
use surreal_derive_plus::surreal_quote;
use surreal_devl::proxy::default::{SurrealDeserializer, SurrealSerializer};
use surreal_devl::surreal_id::SurrealId;
use surreal_devl::surreal_qr::{RPath, SurrealQR, SurrealResponseError};
use surrealdb::sql::Value;
use surrealdb::{engine::local::Db, Surreal};
use surrealdb::sql::Thing;
use uniffi::deps::anyhow;
use crate::entities::token::Token;

#[derive(Clone, Default)]
pub struct TokenId {
    deleted: bool,
    id: u64
}

impl SurrealSerializer for TokenId {
    fn serialize(self) -> Value {
        vec![self.deleted.serialize(), self.id.serialize()].serialize()
    }
}

impl SurrealDeserializer for TokenId {
    fn deserialize(value: &Value) -> Result<Self, SurrealResponseError> {
        match value {
            Value::Array(array) => Ok(Self {
                deleted: SurrealDeserializer::deserialize(&array[0])?,
                id: SurrealDeserializer::deserialize(&array[1])?
            }),
            _ => Err(SurrealResponseError::ExpectedAnArray)
        }
    }
}

impl SurrealId for Token {
    fn id(&self) -> Thing {
        Table::id(self).id(Self::get_table())
    }
}

impl Table<TokenId> for Token {
    fn get_table() -> &'static str {
        "token"
    }

    fn id(&self) -> TokenId {
        TokenId { deleted: false, id: self.order_id }
    }
}

impl DbId for TokenId {
    fn soft_deleted(&self) -> bool {
        self.deleted
    }

    fn soft_delete(&mut self) {
        self.deleted = true;
    }

    fn soft_restore(&mut self) {
        self.deleted = false;
    }
}

pub struct TokenRepository {
    pub db: PoolRequest<Surreal<Db>>
}

#[async_trait::async_trait]
impl LocalSurrealDbRepository<Token, TokenId> for TokenRepository {
    async fn get_db(&self) -> PoolResponse<Surreal<Db>> {
        self.db.retrieve().await.unwrap()
    }
}

impl TokenRepository {
    pub async fn get_latest_token(&self) -> Result<Option<Token>, anyhow::Error> {
        let db = self.get_db().await;
        let token_id = TokenId::default();
        let result: Option<Token> = db
            .query(surreal_quote!("SELECT * FROM #val(&token_id) ORDER BY order_id DESC LIMIT 1")).await?
            .take(RPath::from(0))?;
        Ok(result)
    }
}
