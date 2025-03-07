use core_services::db::repository::abstraction::errors::Resolve;
use core_services::db::repository::abstraction::id::DbId;
use core_services::db::repository::abstraction::local_repository::LocalSurrealDbRepository;
use core_services::db::repository::abstraction::table::Table;
use core_services::utils::pool::reponse::PoolResponse;
use core_services::utils::pool::request::PoolRequest;
use surreal_derive_plus::surreal_quote;
use surreal_devl::proxy::default::{SurrealDeserializer, SurrealSerializer};
use surreal_devl::surreal_id::SurrealId;
use surreal_devl::surreal_qr::{RPath, SurrealResponseError};
use surrealdb::engine::local::Db;
use surrealdb::sql::{Thing, Value};
use surrealdb::Surreal;

use crate::app::transfer::session::TransferSession;

#[derive(Clone)]
pub struct TransferSessionId {
    pub order_id: u64
}

impl SurrealSerializer for TransferSessionId {
    fn serialize(self) -> Value {
        vec![self.order_id.serialize()].serialize()
    }
}

impl SurrealDeserializer for TransferSessionId {
    fn deserialize(value: &Value) -> Result<Self, SurrealResponseError> {
        match value {
            Value::Array(array) => Ok(Self {
                order_id: SurrealDeserializer::deserialize(&array[0])?
            }),
            _ => Err(SurrealResponseError::ExpectedAnArray)
        }
    }
}

impl SurrealId for TransferSession {
    fn id(&self) -> Thing {
        Table::id(self).id(Self::get_table())
    }
}

impl Table<TransferSessionId> for TransferSession {
    fn get_table() -> &'static str {
        "transfer_session"
    }

    fn id(&self) -> TransferSessionId {
        TransferSessionId {
            order_id: self.order_id.clone()
        }
    }
}

impl DbId for TransferSessionId {
    fn soft_deleted(&self) -> bool {
        false
    }

    fn soft_delete(&mut self) {
        panic!("TransferSessionId cannot be soft deleted");
    }

    fn soft_restore(&mut self) {
        panic!("TransferSessionId cannot be soft deleted");
    }
}

pub struct TransferSessionRepository {
    pub db: PoolRequest<Surreal<Db>>
}

#[async_trait::async_trait]
impl LocalSurrealDbRepository<TransferSession, TransferSessionId> for TransferSessionRepository {
    async fn get_db(&self) -> PoolResponse<Surreal<Db>> {
        self.db.retrieve().await.unwrap()
    }
}

impl TransferSessionRepository {
    pub async fn get_last_session(&self) -> Resolve<Option<TransferSession>> {
        let db = self.get_db().await;
        let result: Option<TransferSession> = db
            .query(surreal_quote!("SELECT * FROM transfer_session ORDER BY order_id DESC LIMIT 1"))
            .await?
            .take(RPath::from(0))?;
        Ok(result)
    }
}
