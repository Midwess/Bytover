use core_services::db::repository::abstraction::id::DbId;
use core_services::db::repository::abstraction::repository::Repository;
use core_services::db::repository::abstraction::table::Table;
use surreal_devl::proxy::default::{SurrealDeserializer, SurrealSerializer};
use surreal_devl::surreal_id::SurrealId;
use surreal_devl::surreal_qr::SurrealResponseError;
use surrealdb::sql::{Array, Thing, Value};

use crate::entities::transfer_session::TransferSession;

#[derive(Clone, Default)]
pub struct TransferSessionId {
    pub user_order_id: Option<u64>,
    pub order_id: Option<u64>
}

impl SurrealSerializer for TransferSessionId {
    fn serialize(self) -> Value {
        Value::Array(Array::from(vec![
            self.user_order_id.serialize(),
            self.order_id.serialize(),
        ]))
    }
}

impl SurrealDeserializer for TransferSessionId {
    fn deserialize(value: &Value) -> Result<Self, SurrealResponseError> {
        match value {
            Value::Array(array) => Ok(Self {
                user_order_id: SurrealDeserializer::deserialize(&array.0[0])?,
                order_id: SurrealDeserializer::deserialize(&array.0[1])?
            }),
            _ => Err(SurrealResponseError::ExpectedAnArray)
        }
    }
}

impl Table<TransferSessionId> for TransferSession {
    fn get_table() -> &'static str {
        "transferSession"
    }

    fn id(&self) -> TransferSessionId {
        TransferSessionId {
            user_order_id: Some(self.user_order_id()),
            order_id: Some(self.order_id())
        }
    }
}

impl DbId for TransferSessionId {
    fn soft_delete(&mut self) {
        todo!("Not support soft delete")
    }

    fn soft_restore(&mut self) {
        todo!("Not support soft delete")
    }

    fn soft_deleted(&self) -> bool {
        false
    }
}

impl SurrealId for TransferSession {
    fn id(&self) -> Thing {
        let id = Table::id(self);
        let table = Self::get_table();
        id.id(table)
    }
}

pub trait TransferSessionRepository: Repository<TransferSession, TransferSessionId> {}
