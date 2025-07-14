use std::collections::HashMap;
use std::sync::Arc;
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
use shared::app::file_system::file::{LocalResource, LocalResourcePath};
use shared::app::repository::errors::PersistenceError;
use shared::app::repository::path_resolver::PathResolver;
use shared::app::repository::transfer_session::{TransferSessionId, TransferSessionRepository};
use shared::app::transfer::session::{TransferProgress, TransferSession};
use shared::core_api::{IOReader, IOWriter};
use crate::repository::id::IdbIdWrapper;

pub struct TransferSessionRepositoryImpl {
    pub db: PoolRequest<NeverSend<Database>>,
    pub path_resolver: Arc<dyn PathResolver>
}

impl IdbId for IdbIdWrapper<TransferSessionId> {
    fn deserialize(value: JsValue) -> Result<Self, RepositoryError> {
        let mut json: serde_json::Value = serde_wasm_bindgen::from_value(value)?;
        let table_name = "transferSession";
        if !json.is_array() {
            return Err(RepositoryError::Conflict(table_name.to_owned(), "The id must be an array of primitive types".to_owned()));
        }

        let Some(mut json_array) = json.as_array_mut() else {
            return Err(RepositoryError::Conflict(table_name.to_owned(), "The id must be an array of primitive types".to_owned()));
        };

        Ok(IdbIdWrapper(TransferSessionId {
            r#type: json_array
                .get(0)
                .and_then(|it| serde_json::from_value(it.clone()).ok()),
            target: json_array
                .get(1)
                .and_then(|it| serde_json::from_value(it.clone()).ok()),
            order_id: json_array
                .get(2)
                .and_then(|it| serde_json::from_value(it.clone()).ok())
        }))
    }
}

impl Table<IdbIdWrapper<TransferSessionId>> for TransferSession {
    fn get_table() -> &'static str {
        <Self as Table<TransferSessionId>>::get_table()
    }

    fn id(&self) -> IdbIdWrapper<TransferSessionId> {
        IdbIdWrapper(Table::id(self))
    }
}

impl IdbTable<IdbIdWrapper<TransferSessionId>> for TransferSession {}

#[async_trait::async_trait(?Send)]
impl IdbRepository<TransferSession, IdbIdWrapper<TransferSessionId>> for TransferSessionRepositoryImpl {
    async fn get_db(&self) -> PoolResponse<NeverSend<Database>> {
        self.db.retrieve().await.unwrap()
    }
}

#[async_trait::async_trait(?Send)]
impl Repository<TransferSession, TransferSessionId> for TransferSessionRepositoryImpl {
    async fn create(&self, data: TransferSession) -> Resolve<TransferSession>
    where
        TransferSession: 'async_trait
    {
        IdbRepository::<TransferSession, IdbIdWrapper<TransferSessionId>>::create(self, data).await
    }

    async fn find_one(&self, id: &TransferSessionId) -> Resolve<Option<TransferSession>> {
        IdbRepository::<TransferSession, IdbIdWrapper<TransferSessionId>>::find_one(self, &IdbIdWrapper(id.clone())).await
    }

    async fn find_all(
        &self,
        from_id: Option<&TransferSessionId>,
        to_id: Option<&TransferSessionId>,
        count: Option<usize>
    ) -> Resolve<Vec<TransferSession>> {
        let to_id = to_id.map(|it| IdbIdWrapper(it.clone()));
        IdbRepository::<TransferSession, IdbIdWrapper<TransferSessionId>>::find_all(
            self,
            from_id.map(|it| IdbIdWrapper(it.clone())).as_ref(),
            to_id.as_ref(),
            count
        )
            .await
    }

    async fn delete_one(&self, id: &TransferSessionId) -> Resolve<TransferSession> {
        IdbRepository::<TransferSession, IdbIdWrapper<TransferSessionId>>::delete_one(self, &IdbIdWrapper(id.clone())).await
    }

    async fn update_one(&self, data: TransferSession) -> Resolve<TransferSession> {
        IdbRepository::<TransferSession, IdbIdWrapper<TransferSessionId>>::update_one(self, data).await
    }
}

#[async_trait::async_trait(?Send)]
impl TransferSessionRepository for TransferSessionRepositoryImpl {
    async fn update_progresses(&self, order_id: u64, progresses: Vec<TransferProgress>) -> Result<Option<TransferSession>, PersistenceError> {
        todo!()
    }

    async fn update_resource(&self, session_id: TransferSessionId, resource: LocalResource) -> Result<Option<TransferSession>, PersistenceError> {
        todo!()
    }

    async fn delete_session(&self, session_id: TransferSessionId) -> Result<(), PersistenceError> {
        todo!()
    }

    async fn generate_resource_paths(&self, session_order_id: u64, resource_names: HashMap<u64, String>) -> Result<HashMap<u64, LocalResourcePath>, PersistenceError> {
        todo!()
    }
}
