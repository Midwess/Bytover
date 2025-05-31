use core_services::db::repository::abstraction::errors::Resolve;
use core_services::db::repository::abstraction::id::DbId;
use core_services::db::repository::abstraction::local_repository::LocalSurrealDbRepository;
use core_services::db::repository::abstraction::table::Table;
use core_services::utils::pool::reponse::PoolResponse;
use core_services::utils::pool::request::PoolRequest;
use serde::{Deserialize, Serialize};
use surreal_derive_plus::surreal_quote;
use surreal_devl::proxy::default::{SurrealDeserializer, SurrealSerializer};
use surreal_devl::surreal_id::SurrealId;
use surreal_devl::surreal_qr::{RPath, SurrealResponseError};
use surrealdb::engine::any::Any;
use surrealdb::sql::{Thing, Value};
use surrealdb::Surreal;
use uniffi::Record;

use crate::app::file_system::file::LocalResource;
use crate::app::transfer::session::{TransferProgress, TransferSession, TransferType};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Record)]
pub struct TransferSessionId {
    pub transfer_type: Option<TransferType>,
    pub order_id: Option<u64>
}

impl SurrealSerializer for TransferSessionId {
    fn serialize(self) -> Value {
        vec![
            self.transfer_type.serialize(),
            self.order_id.serialize(),
        ]
        .serialize()
    }
}

impl SurrealDeserializer for TransferSessionId {
    fn deserialize(value: &Value) -> Result<Self, SurrealResponseError> {
        match value {
            Value::Array(array) => Ok(Self {
                transfer_type: SurrealDeserializer::deserialize(&array[0])?,
                order_id: SurrealDeserializer::deserialize(&array[1])?
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
            transfer_type: Some(self.transfer_type.clone()),
            order_id: Some(self.order_id)
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
    pub db: PoolRequest<Surreal<Any>>
}

#[async_trait::async_trait]
impl LocalSurrealDbRepository<TransferSession, TransferSessionId> for TransferSessionRepository {
    async fn get_db(&self) -> PoolResponse<Surreal<Any>> {
        self.db.retrieve().await.unwrap()
    }
}

impl TransferSessionRepository {
    pub async fn save_session(&self, session: TransferSession) -> Resolve<TransferSession> {
        let result = self.create(session).await?;
        Ok(result)
    }

    pub async fn update_progresses(&self, order_id: u64, progresses: Vec<TransferProgress>) -> Resolve<Option<TransferSession>> {
        let session = self
            .find_one(&TransferSessionId {
                order_id: Some(order_id),
                ..Default::default()
            })
            .await?;

        if let Some(session) = session {
            let mut session = session;
            session.progress = progresses;
            let result = self.update_one(session).await?;
            Ok(Some(result))
        } else {
            Ok(None)
        }
    }

    pub async fn get_last_session(&self) -> Resolve<Option<TransferSession>> {
        let db = self.get_db().await;
        let result: Option<TransferSession> = db
            .query(surreal_quote!("SELECT * FROM transfer_session ORDER BY order_id DESC LIMIT 1"))
            .await?
            .take(RPath::from(0))?;
        Ok(result)
    }

    pub async fn update_resource(&self, session_id: TransferSessionId, resource: LocalResource) -> Resolve<Option<TransferSession>> {
        let session = self.find_one(&session_id).await?;
        if let Some(mut session) = session {
            // Find the resource by order_id and update it
            if let Some(existing_resource) = session.resources.iter_mut().find(|r| r.order_id == resource.order_id) {
                *existing_resource = resource;
            }

            log::info!(target: "transfer", "Update resource in database");
            let result = self.update_one(session).await?;
            Ok(Some(result))
        } else {
            Ok(None)
        }
    }
}
