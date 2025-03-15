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

use crate::app::file_system::file::{LocalResource, LocalResourcePath, ResourceType};

#[derive(Clone)]
pub struct LocalResourceId {
    pub r#type: Option<ResourceType>,
    pub order_id: Option<u64>
}

impl SurrealSerializer for LocalResourceId {
    fn serialize(self) -> Value {
        vec![self.r#type.serialize(), self.order_id.serialize()].serialize()
    }
}

impl SurrealDeserializer for LocalResourceId {
    fn deserialize(value: &Value) -> Result<Self, SurrealResponseError> {
        match value {
            Value::Array(array) => Ok(Self {
                r#type: SurrealDeserializer::deserialize(&array[0])?,
                order_id: SurrealDeserializer::deserialize(&array[1])?
            }),
            _ => Err(SurrealResponseError::ExpectedAnArray)
        }
    }
}

impl SurrealId for LocalResource {
    fn id(&self) -> Thing {
        Table::id(self).id(Self::get_table())
    }
}

impl Table<LocalResourceId> for LocalResource {
    fn get_table() -> &'static str {
        "LocalResource"
    }

    fn id(&self) -> LocalResourceId {
        LocalResourceId {
            r#type: Some(self.r#type.clone()),
            order_id: Some(self.order_id)
        }
    }
}

impl DbId for LocalResourceId {
    fn soft_deleted(&self) -> bool {
        false
    }

    fn soft_delete(&mut self) {
        panic!("LocalResourceId cannot be soft deleted");
    }

    fn soft_restore(&mut self) {
        panic!("LocalResourceId cannot be soft deleted");
    }
}

pub struct LocalResourceRepository {
    pub db: PoolRequest<Surreal<Db>>
}

#[async_trait::async_trait]
impl LocalSurrealDbRepository<LocalResource, LocalResourceId> for LocalResourceRepository {
    async fn get_db(&self) -> PoolResponse<Surreal<Db>> {
        self.db.retrieve().await.unwrap()
    }
}

impl LocalResourceRepository {
    pub async fn find_by_path(&self, path: &LocalResourcePath) -> Option<LocalResource> {
        let db = self.get_db().await;
        let resources: Option<LocalResource> = db.query(surreal_quote!("SELECT * FROM LocalResource WHERE path = #val(path)"))
            .await
            .expect("Failed to connect to local resource database")
            .take(RPath::from(0))
            .unwrap();

        resources
    }
}
