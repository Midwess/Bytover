use crate::core_api_impl::io::IOReaderImpl;
use crate::file_api::opfs::{IOReaderOpfsImpl, IOWriterOpfsImpl, OPFS_WORKER};
use crate::file_api::path_extension::WebExtLocalResourcePath;
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
use shared::app::repository::errors::PersistenceError;
use shared::app::repository::local_resource::{LocalResourceId, LocalResourceRepository};
use shared::core_api::{IOReader, IOWriter};
use shared::entities::file_system::file::{LocalResource, LocalResourcePath, ResourceType};
use std::collections::HashMap;
use wasm_bindgen::JsValue;
use crate::deserialize;
use crate::file_api::file_extension::VecExtension;
use crate::web_worker::bridge::WorkerMessage;
use crate::web_worker::opfs::{FileOperation, OpfsOperation, OpfsOperationOutput};

pub struct LocalResourceRepositoryImpl {
    pub db: PoolRequest<NeverSend<Database>>,
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
        let Some(path) = path.opfs_path() else {
            return Ok(None);
        };

        let resp = OPFS_WORKER.send(WorkerMessage::new(OpfsOperation {
            file_path: path,
            operation: FileOperation::LocalResourceInstance
        })).await.unwrap().message;

        let OpfsOperationOutput::LocalResourceInstance(resource) = resp else {
            return Ok(None);
        };

        Ok(Some(deserialize(&resource)))
    }

    async fn save_thumbnail(&self, png_bytes: Vec<u8>, resource_id: u64) -> Result<LocalResourcePath, PersistenceError> {
        let save_path = LocalResourcePath::resource_thumbnail(None, resource_id);
        let path = save_path.opfs_path().unwrap();

        OPFS_WORKER.send(WorkerMessage::new(OpfsOperation {
            file_path: path,
            operation: FileOperation::WriteNew {
                data: png_bytes.into_uint_array_leak()
            }
        })).await;
        Ok(save_path)
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

    async fn read(&self, path: LocalResourcePath, chunk_size: usize) -> Result<Box<dyn IOReader>, PersistenceError> {
        if let Some(path) = path.opfs_path() {
            let reader = IOReaderOpfsImpl::new(path.into()).await?;
            return Ok(Box::new(reader))
        }

        Err(PersistenceError::NotFound(format!("{:?}", path)))
    }

    async fn write(&self, path: LocalResourcePath) -> Result<Box<dyn IOWriter>, PersistenceError> {
        if let Some(path) = path.opfs_path() {
            let writer = IOWriterOpfsImpl::new(path.into()).await?;
            return Ok(Box::new(writer));
        }

        Err(PersistenceError::NotFound(format!("Your path is not supported {:?}", path)))
    }

    async fn generate_thumbnail_paths(
        &self,
        session_id: Option<u64>,
        resource_ids: Vec<u64>
    ) -> Result<HashMap<u64, LocalResourcePath>, PersistenceError> {
        let mut result = HashMap::new();
        for resource_id in resource_ids.iter() {
            let path = LocalResourcePath::resource_thumbnail(session_id, *resource_id);
            result.insert(*resource_id, path);
        }

        Ok(result)
    }

    async fn size(&self, path: LocalResourcePath) -> Result<u64, PersistenceError> {
        let reader = self.read(path.clone(), 0).await?;
        Ok(reader.entry().await?.size)
    }
}
