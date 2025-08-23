use crate::core_api_impl::io::IOReaderImpl;
use crate::file_api::cache::{BrowserCache, CacheResource, IOReaderBrowserCacheImpl};
use crate::file_api::path_extension::WebExtLocalResourcePath;
use crate::file_api::storage::FileStorage;
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
use futures::lock::Mutex;
use idb::Database;
use shared::app::file_system::file::{LocalResource, LocalResourcePath, ResourceType};
use shared::app::repository::errors::PersistenceError;
use shared::app::repository::local_resource::{LocalResourceId, LocalResourceRepository};
use shared::core_api::{IOReader, IOWriter};
use std::collections::HashMap;
use wasm_bindgen::JsValue;

pub struct LocalResourceRepositoryImpl {
    pub db: PoolRequest<NeverSend<Database>>,
    pub file_storage: FileStorage,
    pub thumbnail_caches: Mutex<HashMap<u64, IOReaderBrowserCacheImpl>>,
    pub resource_caches: Mutex<HashMap<u64, IOReaderBrowserCacheImpl>>
}

impl IdbId for IdbIdWrapper<LocalResourceId> {
    fn deserialize(value: JsValue) -> Result<Self, RepositoryError> {
        let mut json: serde_json::Value = serde_wasm_bindgen::from_value(value)?;
        let table_name = "localResource";
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

        Ok(IdbIdWrapper(LocalResourceId {
            r#type: json_array.first().and_then(|it| serde_json::from_value(it.clone()).ok()),
            path: json_array.get(1).and_then(|it| serde_json::from_value(it.clone()).ok()),
            order_id: json_array.get(2).and_then(|it| it.as_str().and_then(|it| it.parse().ok()))
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
        let Some(path) = path.device_file_id() else {
            return Ok(None);
        };

        let Some(resource) = self.file_storage.get(path).await else {
            return Ok(None);
        };

        Ok(Some(resource.resource))
    }

    async fn save_thumbnail(&self, png_bytes: Vec<u8>, resource_id: u64) -> Result<LocalResourcePath, PersistenceError> {
        let key = resource_id.to_string();
        let mut thumbnail_caches = self.thumbnail_caches.lock().await;
        let new_resource = CacheResource::thumbnail(resource_id);
        let (mut writer, reader) = BrowserCache::create(self.db.clone(), new_resource).await?;
        thumbnail_caches.insert(resource_id, reader);

        writer.write(png_bytes.into()).await?;
        writer.end().await?;
        let saved_path = LocalResourcePath::cache("thumbnails", key);
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
        if let Some(device_file_id) = path.device_file_id() {
            let Some(file) = self.file_storage.get(device_file_id).await else {
                return Err(PersistenceError::NotFound(format!("{:?}", path)));
            };

            return Ok(Box::new(IOReaderImpl {
                file: Mutex::new(file.file.clone()),
                position: 0,
                chunk_size: 63 * 1024
            }))
        };

        if let Some(key) = path.thumbnail_resource_id() {
            let thumbnail_caches = self.thumbnail_caches.lock().await;
            let Some(cache) = thumbnail_caches.get(&key) else {
                return Err(PersistenceError::NotFound(format!("{:?}", path)));
            };

            return Ok(Box::new(cache.try_clone().await?))
        }

        if let Some(key) = path.resource_id() {
            let resource_caches = self.resource_caches.lock().await;
            let Some(cache) = resource_caches.get(&key) else {
                return Err(PersistenceError::NotFound(format!("{:?}", path)));
            };

            return Ok(Box::new(cache.try_clone().await?))
        }

        Err(PersistenceError::NotFound(format!("{:?}", path)))
    }

    async fn write(&self, path: LocalResourcePath) -> Result<Box<dyn IOWriter>, PersistenceError> {
        if let Some(resource_id) = path.resource_id() {
            let mut caches = self.resource_caches.lock().await;
            let resource = CacheResource::resource(resource_id);
            let (writer, reader) = BrowserCache::create(self.db.clone(), resource).await?;
            let writer: Box<dyn IOWriter> = Box::new(writer);
            caches.insert(resource_id, reader);

            return Ok(writer);
        }

        if let Some(thumbnail_resource_id) = path.thumbnail_resource_id() {
            let mut caches = self.thumbnail_caches.lock().await;
            let resource = CacheResource::thumbnail(thumbnail_resource_id);
            let (writer, reader) = BrowserCache::create(self.db.clone(), resource).await?;
            let writer: Box<dyn IOWriter> = Box::new(writer);
            caches.insert(thumbnail_resource_id, reader);
            return Ok(writer);
        }

        Err(PersistenceError::NotFound(format!("Your path is not supported {:?}", path)))
    }

    async fn generate_thumbnail_paths(&self, resource_ids: Vec<u64>) -> Result<HashMap<u64, LocalResourcePath>, PersistenceError> {
        let mut result = HashMap::new();
        for resource_id in resource_ids.iter() {
            let path = LocalResourcePath::cache("thumbnails", resource_id.to_string());
            result.insert(*resource_id, path);
        }

        Ok(result)
    }

    async fn size(&self, path: LocalResourcePath) -> Result<u64, PersistenceError> {
        let reader = self.read(path.clone(), 0).await?;
        Ok(reader.total_size().await?)
    }
}
