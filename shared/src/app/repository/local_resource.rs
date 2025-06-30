use crate::app::file_system::file::{LocalResource, LocalResourcePath, ResourceType};
use core_services::db::repository::abstraction::id::DbId;
use core_services::db::repository::abstraction::repository::Repository;
use core_services::db::repository::abstraction::table::Table;
use serde::{Deserialize, Serialize};
use url::Url;
use crate::app::repository::errors::PersistenceError;
use crate::core_api::{IOReader, IOWriter, NetStream};

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct LocalResourceId {
    pub r#type: Option<ResourceType>,
    pub path: Option<LocalResourcePath>,
    pub order_id: Option<u64>
}

#[async_trait::async_trait]
pub trait LocalResourceRepository: Repository<LocalResource, LocalResourceId> {
    async fn load(&self, path: LocalResourcePath) -> Result<Option<LocalResource>, PersistenceError>;
    async fn save_thumbnail(&self, png_bytes: Vec<u8>, resource_id: u64) -> Result<LocalResourcePath, PersistenceError>;
    async fn get_resource_type(&self, path: LocalResourcePath) -> Result<ResourceType, PersistenceError>;
    async fn load_all(&self) -> Result<Vec<LocalResource>, PersistenceError>;
    async fn read(&self, path: LocalResourcePath, max_length: usize) -> Result<Box<dyn IOReader>, PersistenceError>;
    async fn write(&self, path: LocalResourcePath) -> Result<Box<dyn IOWriter>, PersistenceError>;
    async fn new_thumbnail_writer(&self, resource_id: u64) -> Result<(Box<dyn IOWriter>, LocalResourcePath), PersistenceError>;
}

impl Table<LocalResourceId> for LocalResource {
    fn get_table() -> &'static str {
        "localResource"
    }

    fn id(&self) -> LocalResourceId {
        LocalResourceId {
            r#type: Some(self.r#type.clone()),
            path: Some(self.path.clone()),
            order_id: Some(self.order_id)
        }
    }
}

impl DbId for LocalResourceId {
    fn soft_deleted(&self) -> bool {
        false
    }

    fn soft_delete(&mut self) {}

    fn soft_restore(&mut self) {}
}
