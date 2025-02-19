use crate::entities::user::User;
use core_services::db::repository::abstraction::id::DbId;
use core_services::db::repository::abstraction::local_repository::LocalSurrealDbRepository;
use core_services::db::repository::abstraction::table::Table;
use core_services::utils::pool::reponse::PoolResponse;
use core_services::utils::pool::request::PoolRequest;
use surreal_devl::proxy::default::{SurrealDeserializer, SurrealSerializer};
use surreal_devl::surreal_id::SurrealId;
use surreal_devl::surreal_qr::SurrealResponseError;
use surrealdb::engine::local::Db;
use surrealdb::sql::{Thing, Value};
use surrealdb::Surreal;

#[derive(Clone, Default)]
pub struct UserId {
    email: String
}

impl SurrealSerializer for UserId {
    fn serialize(self) -> Value {
        vec![self.email.serialize()].serialize()
    }
}

impl SurrealDeserializer for UserId {
    fn deserialize(value: &Value) -> Result<Self, SurrealResponseError> {
        match value {
            Value::Array(array) => Ok(Self {
                email: SurrealDeserializer::deserialize(&array[0])?
            }),
            _ => Err(SurrealResponseError::ExpectedAnArray)
        }
    }
}

impl SurrealId for User {
    fn id(&self) -> Thing {
        Table::id(self).id(Self::get_table())
    }
}

impl Table<UserId> for User {
    fn get_table() -> &'static str {
        "user"
    }

    fn id(&self) -> UserId {
        UserId {
            email: self.email.clone()
        }
    }
}

impl DbId for UserId {
    fn soft_deleted(&self) -> bool {
        false
    }

    fn soft_delete(&mut self) {
        panic!("User cannot be soft deleted");
    }

    fn soft_restore(&mut self) {
        panic!("User cannot be soft deleted");
    }
}

pub struct TokenRepository {
    pub db: PoolRequest<Surreal<Db>>
}

#[warn(dead_code)]
pub struct UserRepository {
    db: PoolRequest<Surreal<Db>>
}

#[async_trait::async_trait]
impl LocalSurrealDbRepository<User, UserId> for UserRepository {
    async fn get_db(&self) -> PoolResponse<Surreal<Db>> {
        self.db.retrieve().await.unwrap()
    }
}
