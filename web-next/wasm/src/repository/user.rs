use core_services::db::idb::id::IdbId;
use core_services::db::idb::repository::IdbRepository;
use core_services::db::idb::table::IdbTable;
use core_services::db::repository::abstraction::errors::{RepositoryError, Resolve};
use core_services::db::repository::abstraction::repository::Repository;
use core_services::db::repository::abstraction::table::Table;
use idb::Database;
use wasm_bindgen::JsValue;
use core_services::utils::never_send::NeverSend;
use core_services::utils::pool::reponse::PoolResponse;
use core_services::utils::pool::request::PoolRequest;
use shared::app::repository::user::{UserId, UserRepository};
use shared::entities::user::User;
use crate::repository::id::IdbIdWrapper;

pub struct UserRepositoryImpl {
    pub db: PoolRequest<NeverSend<Database>>
}

impl IdbId for IdbIdWrapper<UserId> {
    fn deserialize(value: JsValue) -> Result<Self, RepositoryError> {
        let mut json: serde_json::Value = serde_wasm_bindgen::from_value(value)?;
        let table_name = "user";
        if !json.is_array() {
            return Err(RepositoryError::Conflict(table_name.to_owned(), "The id must be an array of primitive types".to_owned()));
        }

        let Some(mut json_array) = json.as_array_mut() else {
            return Err(RepositoryError::Conflict(table_name.to_owned(), "The id must be an array of primitive types".to_owned()));
        };

        let Some(email) = json_array
            .get(1)
            .and_then(|it| serde_json::from_value(it.clone()).ok()) else {
            return Err(RepositoryError::Conflict(table_name.to_owned(), "The id must be an array of primitive types".to_owned()));
        };

        Ok(IdbIdWrapper(UserId {
            email
        }))
    }
}

impl Table<IdbIdWrapper<UserId>> for User {
    fn get_table() -> &'static str {
        <Self as Table<UserId>>::get_table()
    }

    fn id(&self) -> IdbIdWrapper<UserId> {
        IdbIdWrapper(Table::id(self))
    }
}

impl IdbTable<IdbIdWrapper<UserId>> for User {}

#[async_trait::async_trait(?Send)]
impl IdbRepository<User, IdbIdWrapper<UserId>> for UserRepositoryImpl {
    async fn get_db(&self) -> PoolResponse<NeverSend<Database>> {
        self.db.retrieve().await.unwrap()
    }
}

#[async_trait::async_trait(?Send)]
impl Repository<User, UserId> for UserRepositoryImpl {
    async fn create(&self, data: User) -> Resolve<User>
    where
        User: 'async_trait
    {
        IdbRepository::<User, IdbIdWrapper<UserId>>::create(self, data).await
    }

    async fn find_one(&self, id: &UserId) -> Resolve<Option<User>> {
        IdbRepository::<User, IdbIdWrapper<UserId>>::find_one(self, &IdbIdWrapper(id.clone())).await
    }

    async fn find_all(
        &self,
        from_id: Option<&UserId>,
        to_id: Option<&UserId>,
        count: Option<usize>
    ) -> Resolve<Vec<User>> {
        let to_id = to_id.map(|it| IdbIdWrapper(it.clone()));
        IdbRepository::<User, IdbIdWrapper<UserId>>::find_all(
            self,
            from_id.map(|it| IdbIdWrapper(it.clone())).as_ref(),
            to_id.as_ref(),
            count
        )
            .await
    }

    async fn delete_one(&self, id: &UserId) -> Resolve<User> {
        IdbRepository::<User, IdbIdWrapper<UserId>>::delete_one(self, &IdbIdWrapper(id.clone())).await
    }

    async fn update_one(&self, data: User) -> Resolve<User> {
        IdbRepository::<User, IdbIdWrapper<UserId>>::update_one(self, data).await
    }
}

#[async_trait::async_trait(?Send)]
impl UserRepository for UserRepositoryImpl {}
