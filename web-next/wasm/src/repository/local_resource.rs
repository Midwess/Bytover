use std::collections::HashMap;
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
use shared::app::file_system::file::{LocalResource, LocalResourcePath, ResourceType};
use shared::app::repository::errors::PersistenceError;
use shared::app::repository::local_resource::{LocalResourceId, LocalResourceRepository};
use shared::core_api::{IOReader, IOWriter};
use crate::repository::id::IdbIdWrapper;

pub struct LocalResourceRepositoryImpl {
    pub db: PoolRequest<NeverSend<Database>>
}

impl IdbId for IdbIdWrapper<LocalResourceId> {
    fn deserialize(value: JsValue) -> Result<Self, RepositoryError> {
        let mut json: serde_json::Value = serde_wasm_bindgen::from_value(value)?;
        let table_name = "localResource";
        if !json.is_array() {
            return Err(RepositoryError::Conflict(table_name.to_owned(), "The id must be an array of primitive types".to_owned()));
        }

        let Some(mut json_array) = json.as_array_mut() else {
            return Err(RepositoryError::Conflict(table_name.to_owned(), "The id must be an array of primitive types".to_owned()));
        };

        Ok(IdbIdWrapper(LocalResourceId {
            r#type: json_array
                .get(0)
                .and_then(|it| serde_json::from_value(it.clone()).ok()),
            path: json_array
                .get(1)
                .and_then(|it| serde_json::from_value(it.clone()).ok()),
            order_id: json_array
                .get(2)
                .and_then(|it| serde_json::from_value(it.clone()).ok())
        }))
    }
}

impl Table<IdbIdWrapper<LocalResourceId>> for LocalResource {
    fn get_table() -> &'static str {
        <Self as Table<LocalResourceId>>::get_table()
    }

    fn id(&self) -> IdbIdWrapper<LocalResourceId> {
        IdbIdWrapper(Table::id(self))
    }
}

impl IdbTable<IdbIdWrapper<LocalResourceId>> for LocalResource {}

#[async_trait::async_trait(?Send)]
impl IdbRepository<LocalResource, IdbIdWrapper<LocalResourceId>> for LocalResourceRepositoryImpl {
    async fn get_db(&self) -> PoolResponse<NeverSend<Database>> {
        self.db.retrieve().await.unwrap()
    }
}

#[async_trait::async_trait(?Send)]
impl Repository<LocalResource, LocalResourceId> for LocalResourceRepositoryImpl {
    async fn create(&self, data: LocalResource) -> Resolve<LocalResource>
    where
        LocalResource: 'async_trait
    {
        IdbRepository::<LocalResource, IdbIdWrapper<LocalResourceId>>::create(self, data).await
    }

    async fn find_one(&self, id: &LocalResourceId) -> Resolve<Option<LocalResource>> {
        IdbRepository::<LocalResource, IdbIdWrapper<LocalResourceId>>::find_one(self, &IdbIdWrapper(id.clone())).await
    }

    async fn find_all(
        &self,
        from_id: Option<&LocalResourceId>,
        to_id: Option<&LocalResourceId>,
        count: Option<usize>
    ) -> Resolve<Vec<LocalResource>> {
        let to_id = to_id.map(|it| IdbIdWrapper(it.clone()));
        IdbRepository::<LocalResource, IdbIdWrapper<LocalResourceId>>::find_all(
            self,
            from_id.map(|it| IdbIdWrapper(it.clone())).as_ref(),
            to_id.as_ref(),
            count
        )
            .await
    }

    async fn delete_one(&self, id: &LocalResourceId) -> Resolve<LocalResource> {
        IdbRepository::<LocalResource, IdbIdWrapper<LocalResourceId>>::delete_one(self, &IdbIdWrapper(id.clone())).await
    }

    async fn update_one(&self, data: LocalResource) -> Resolve<LocalResource> {
        IdbRepository::<LocalResource, IdbIdWrapper<LocalResourceId>>::update_one(self, data).await
    }
}

#[async_trait::async_trait(?Send)]
impl LocalResourceRepository for LocalResourceRepositoryImpl {
    async fn load(&self, path: LocalResourcePath) -> Result<Option<LocalResource>, PersistenceError> {
        todo!()
    }

    async fn save_thumbnail(&self, png_bytes: Vec<u8>, resource_id: u64) -> Result<LocalResourcePath, PersistenceError> {
        todo!()
    }

    async fn get_resource_type(&self, path: LocalResourcePath) -> Result<ResourceType, PersistenceError> {
        todo!()
    }

    async fn load_all(&self) -> Result<Vec<LocalResource>, PersistenceError> {
        todo!()
    }

    async fn read(&self, path: LocalResourcePath, max_length: usize) -> Result<Box<dyn IOReader>, PersistenceError> {
        todo!()
    }

    async fn write(&self, path: LocalResourcePath) -> Result<Box<dyn IOWriter>, PersistenceError> {
        todo!()
    }

    async fn new_thumbnail_writer(&self, resource_id: u64) -> Result<(Box<dyn IOWriter>, LocalResourcePath), PersistenceError> {
        todo!()
    }

    async fn generate_thumbnail_paths(&self, resource_ids: Vec<u64>) -> Result<HashMap<u64, LocalResourcePath>, PersistenceError> {
        todo!()
    }
}
