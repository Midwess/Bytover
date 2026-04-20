use core_services::db::idb::id::IdbId;
use core_services::db::idb::repository::IdbRepository;
use core_services::db::idb::table::IdbTable;
use core_services::db::repository::abstraction::errors::{RepositoryError, Resolve};
use core_services::db::repository::abstraction::repository::Repository;
use core_services::db::repository::abstraction::table::Table;
use core_services::utils::never_send::NeverSend;
use core_services::utils::pool::reponse::PoolResponse;
use core_services::utils::pool::request::PoolRequest;
use idb::Database;
use shared::entities::session::Session;
use shared::repository::auth_session::{AuthSessionId, AuthSessionRepository};
use wasm_bindgen::JsValue;

use crate::repository::id::IdbIdWrapper;

pub struct AuthSessionRepositoryImpl {
    pub db: PoolRequest<NeverSend<Database>>,
}

impl IdbId for IdbIdWrapper<AuthSessionId> {
    fn deserialize(value: JsValue) -> Result<Self, RepositoryError> {
        let mut json: serde_json::Value = serde_wasm_bindgen::from_value(value)?;
        let table_name = "authSession";
        if !json.is_array() {
            return Err(RepositoryError::Conflict(
                table_name.to_owned(),
                "The id must be an array of primitive types".to_owned(),
            ));
        }

        let Some(json_array) = json.as_array_mut() else {
            return Err(RepositoryError::Conflict(
                table_name.to_owned(),
                "The id must be an array of primitive types".to_owned(),
            ));
        };

        let Some(r#type) = json_array.first().and_then(|it| serde_json::from_value(it.clone()).ok()) else {
            return Err(RepositoryError::Conflict(
                table_name.to_owned(),
                "Missing type in id".to_owned(),
            ));
        };

        Ok(IdbIdWrapper(AuthSessionId { r#type }))
    }
}

impl Table<IdbIdWrapper<AuthSessionId>> for Session {
    fn get_table() -> &'static str {
        <Self as Table<AuthSessionId>>::get_table()
    }

    fn id(&self) -> IdbIdWrapper<AuthSessionId> {
        IdbIdWrapper(Table::id(self))
    }
}

impl IdbTable<IdbIdWrapper<AuthSessionId>> for Session {}

#[async_trait::async_trait(?Send)]
impl IdbRepository<Session, IdbIdWrapper<AuthSessionId>> for AuthSessionRepositoryImpl {
    async fn get_db(&self) -> PoolResponse<NeverSend<Database>> {
        self.db.retrieve().await.unwrap()
    }
}

#[async_trait::async_trait(?Send)]
impl Repository<Session, AuthSessionId> for AuthSessionRepositoryImpl {
    async fn create(&self, data: Session) -> Resolve<Session>
    where
        Session: 'async_trait,
    {
        IdbRepository::<Session, IdbIdWrapper<AuthSessionId>>::create(self, data).await
    }

    async fn find_one(&self, id: &AuthSessionId) -> Resolve<Option<Session>> {
        IdbRepository::<Session, IdbIdWrapper<AuthSessionId>>::find_one(self, &IdbIdWrapper(id.clone())).await
    }

    async fn find_all(
        &self,
        from_id: Option<&AuthSessionId>,
        to_id: Option<&AuthSessionId>,
        count: Option<usize>,
    ) -> Resolve<Vec<Session>> {
        let to_id = to_id.map(|it| IdbIdWrapper(it.clone()));
        IdbRepository::<Session, IdbIdWrapper<AuthSessionId>>::find_all(
            self,
            from_id.map(|it| IdbIdWrapper(it.clone())).as_ref(),
            to_id.as_ref(),
            count,
        )
        .await
    }

    async fn delete_one(&self, id: &AuthSessionId) -> Resolve<Session> {
        IdbRepository::<Session, IdbIdWrapper<AuthSessionId>>::delete_one(self, &IdbIdWrapper(id.clone())).await
    }

    async fn update_one(&self, data: Session) -> Resolve<Session> {
        IdbRepository::<Session, IdbIdWrapper<AuthSessionId>>::update_one(self, data).await
    }
}

#[async_trait::async_trait]
impl AuthSessionRepository for AuthSessionRepositoryImpl {}
