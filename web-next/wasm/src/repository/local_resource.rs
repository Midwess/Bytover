use std::collections::HashMap;
use std::sync::Arc;
use futures::lock::Mutex;
use core_services::db::idb::id::IdbId;
use core_services::db::idb::repository::IdbRepository;
use core_services::db::idb::table::IdbTable;
use core_services::db::repository::abstraction::errors::{RepositoryError, Resolve};
use core_services::db::repository::abstraction::repository::Repository;
use core_services::db::repository::abstraction::table::Table;
use idb::{Database, TransactionMode};
use wasm_bindgen::JsValue;
use core_services::utils::never_send::NeverSend;
use core_services::utils::pool::reponse::PoolResponse;
use core_services::utils::pool::request::PoolRequest;
use shared::app::file_system::file::{LocalResource, LocalResourcePath, ResourceType};
use shared::app::repository::errors::PersistenceError;
use shared::app::repository::local_resource::{LocalResourceId, LocalResourceRepository};
use shared::core_api::{IOReader, IOWriter};
use crate::core_api_impl::io::IOReaderImpl;
use crate::file_api::storage::{FileStorage, WasmFile};
use crate::repository::id::IdbIdWrapper;

pub struct LocalResourceRepositoryImpl {
    pub db: PoolRequest<NeverSend<Database>>,
    pub file_storage: FileStorage
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
                .and_then(|it| it.as_str().and_then(|it| it.parse().ok()))
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
        // On web, we don't save any local resource
        // because the path will not be correct after reload
        Ok(data)
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
        let Some(resource) = self.file_storage.load(path).await else {
            return Ok(None);
        };

        Ok(Some(resource))
    }

    async fn save_thumbnail(&self, png_bytes: Vec<u8>, resource_id: u64) -> Result<LocalResourcePath, PersistenceError> {
        let Some(saved_path) = self.file_storage.save_thumbnail(resource_id, png_bytes).await else {
            return Err(PersistenceError::IOError("Unable to save thumbnail".to_owned()))
        };

        Ok(saved_path)
    }

    async fn get_resource_type(&self, path: LocalResourcePath) -> Result<ResourceType, PersistenceError> {
        let Some(resource) = self.load(path.clone()).await? else {
            return Err(PersistenceError::NotFound(format!("{:?}", path)));
        };

        Ok(resource.r#type)
    }

    async fn load_all(&self) -> Result<Vec<LocalResource>, PersistenceError> {
        Ok(vec![])
    }

    async fn read(&self, path: LocalResourcePath, _max_length: usize) -> Result<Box<dyn IOReader>, PersistenceError> {
        let Some(file) = self.file_storage.get_file_by_path(&path).await else {
            return Err(PersistenceError::NotFound(format!("{:?}", path)));
        };

        Ok(Box::new(IOReaderImpl {
            file: Mutex::new(WasmFile(file)),
            position: 0,
            chunk_size: 63 * 1024
        }))
    }

    async fn write(&self, path: LocalResourcePath) -> Result<Box<dyn IOWriter>, PersistenceError> {
        todo!()
    }

    async fn new_thumbnail_writer(&self, resource_id: u64) -> Result<(Box<dyn IOWriter>, LocalResourcePath), PersistenceError> {
        todo!()
    }

    async fn generate_thumbnail_paths(&self, resource_ids: Vec<u64>) -> Result<HashMap<u64, LocalResourcePath>, PersistenceError> {
        let mut result = HashMap::new();
        for resource_id in resource_ids.iter() {
            let path = LocalResourcePath::PlatformIdentifier(format!("idb://thumbnail/{resource_id}"));
            result.insert(*resource_id, path);
        }

        Ok(result)
    }

    async fn size(&self, path: LocalResourcePath) -> Result<u64, PersistenceError> {
        let LocalResourcePath::PlatformIdentifier(path) = path else {
            return Err(PersistenceError::NotFound(format!("Expected platform identifier got {:?}", path)));
        };

        if let Ok(resource_id) = path.trim_start_matches("idb://thumbnail/").parse::<u64>() {
            if let Some(resource) = self.file_storage.read_thumbnail_bytes(resource_id).await {
                return Ok(resource.length() as u64);
            };
        };

        if let Some(file) = self.file_storage.get_file_by_path(&LocalResourcePath::PlatformIdentifier(path.clone())).await {
            return Ok(file.size() as u64)
        };

        Err(PersistenceError::NotFound(format!("Not found file at path {path:?}")))
    }
}
