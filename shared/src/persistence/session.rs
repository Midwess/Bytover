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
use crate::entities::session::{Session, SessionType};
use crate::entities::token::Token;

#[derive(Clone)]
pub struct SessionId {
    pub r#type: SessionType
}

impl SurrealSerializer for SessionId {
    fn serialize(self) -> Value {
        vec![self.r#type.serialize()].serialize()
    }
}

impl SurrealDeserializer for SessionId {
    fn deserialize(value: &Value) -> Result<Self, SurrealResponseError> {
        match value {
            Value::Array(array) => Ok(Self {
                r#type: SurrealDeserializer::deserialize(&array[0])?
            }),
            _ => Err(SurrealResponseError::ExpectedAnArray)
        }
    }
}

impl SurrealId for Session {
    fn id(&self) -> Thing {
        Table::id(self).id(Self::get_table())
    }
}

impl Table<SessionId> for Session {
    fn get_table() -> &'static str {
        "session"
    }

    fn id(&self) -> SessionId {
        SessionId { r#type: self.r#type.clone() }
    }
}

impl DbId for SessionId {
    fn soft_deleted(&self) -> bool {
        false
    }

    fn soft_delete(&mut self) {
        panic!("SessionId cannot be soft deleted");
    }

    fn soft_restore(&mut self) {
        panic!("SessionId cannot be soft deleted");
    }
}

pub struct SessionRepository {
    pub db: PoolRequest<Surreal<Db>>
}

#[async_trait::async_trait]
impl LocalSurrealDbRepository<Session, SessionId> for SessionRepository {
    async fn get_db(&self) -> PoolResponse<Surreal<Db>> {
        self.db.retrieve().await.unwrap()
    }
}
