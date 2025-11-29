use crate::file_system::path_extension::WebExtLocalResourcePath;
use crate::repository::id::IdbIdWrapper;
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
use shared::entities::local_resource::{LocalResource, LocalResourcePath};
use shared::entities::transfer_session::{TransferProgress, TransferSession};
use shared::repository::errors::PersistenceError;
use shared::repository::transfer_session::{TransferSessionId, TransferSessionRepository};
use std::collections::HashMap;
use wasm_bindgen::JsValue;

pub struct TransferSessionRepositoryImpl {
    pub db: PoolRequest<NeverSend<Database>>
}

impl IdbId for IdbIdWrapper<TransferSessionId> {
    fn deserialize(value: JsValue) -> Result<Self, RepositoryError> {
        let mut json: serde_json::Value = serde_wasm_bindgen::from_value(value)?;
        let table_name = "transferSession";
        if !json.is_array() {
            return Err(RepositoryError::Conflict(
                table_name.to_owned(),
                "The id must be an array of primitive types".to_owned()
            ));
        }

        let Some(json_array) = json.as_array_mut() else {
            return Err(RepositoryError::Conflict(
                table_name.to_owned(),
                "The id must be an array of primitive types".to_owned()
            ));
        };

        Ok(IdbIdWrapper(TransferSessionId {
            transfer_type: json_array.first().and_then(|it| serde_json::from_value(it.clone()).ok()),
            order_id: json_array.get(1).and_then(|v| v.as_str().to_owned()).map(|it| it.to_string())
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
        log::info!("create session: {:?}", data);
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
        let returned = IdbRepository::<TransferSession, IdbIdWrapper<TransferSessionId>>::delete_one(self, &IdbIdWrapper(id.clone())).await?;
        Ok(returned)
    }

    async fn update_one(&self, data: TransferSession) -> Resolve<TransferSession> {
        IdbRepository::<TransferSession, IdbIdWrapper<TransferSessionId>>::update_one(self, data).await
    }
}

#[async_trait::async_trait(?Send)]
impl TransferSessionRepository for TransferSessionRepositoryImpl {
    async fn update_progresses(
        &self,
        order_id: u64,
        progresses: Vec<TransferProgress>
    ) -> Result<Option<TransferSession>, PersistenceError> {
        let id = TransferSessionId {
            order_id: Some(order_id.to_string()),
            ..Default::default()
        };

        let session = IdbRepository::<TransferSession, IdbIdWrapper<TransferSessionId>>::find_one(self, &IdbIdWrapper(id)).await?;

        if let Some(session) = session {
            let mut session = session;
            session.progress = progresses;
            let result = IdbRepository::<TransferSession, IdbIdWrapper<TransferSessionId>>::update_one(self, session).await?;
            Ok(Some(result))
        } else {
            Ok(None)
        }
    }

    async fn update_resource(
        &self,
        session_id: TransferSessionId,
        resource: LocalResource
    ) -> Result<Option<TransferSession>, PersistenceError> {
        log::info!("update_resource of session: {:?}", session_id);
        let session =
            IdbRepository::<TransferSession, IdbIdWrapper<TransferSessionId>>::find_one(self, &IdbIdWrapper(session_id.clone()))
                .await?;

        if let Some(mut session) = session {
            session.replace_resource(resource);
            let result = IdbRepository::<TransferSession, IdbIdWrapper<TransferSessionId>>::update_one(self, session).await?;
            Ok(Some(result))
        } else {
            Ok(None)
        }
    }

    async fn delete_session(&self, session_id: TransferSessionId) -> Result<(), PersistenceError> {
        IdbRepository::<TransferSession, IdbIdWrapper<TransferSessionId>>::delete_one(self, &IdbIdWrapper(session_id)).await?;

        Ok(())
    }

    async fn generate_resource_paths(
        &self,
        session_order_id: u64,
        resource_names: HashMap<u64, String>
    ) -> Result<HashMap<u64, LocalResourcePath>, PersistenceError> {
        let mut result = HashMap::new();

        for (resource_order_id, resource_name) in resource_names {
            let extension = resource_name.split('.').next_back().unwrap_or("unknown").to_string();
            result.insert(
                resource_order_id,
                LocalResourcePath::session_resource(session_order_id, resource_order_id, extension)
            );
        }

        Ok(result)
    }
}
