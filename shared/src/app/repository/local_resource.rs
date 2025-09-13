use crate::app::repository::errors::PersistenceError;
use crate::core_api::{IOReader, IOWriter};
use crate::entities::file_system::file::{LocalResource, LocalResourcePath, ResourceType};
use core_services::db::repository::abstraction::id::DbId;
use core_services::db::repository::abstraction::repository::Repository;
use core_services::db::repository::abstraction::table::Table;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct LocalResourceId {
    pub r#type: Option<ResourceType>,
    pub path: Option<String>,
    pub order_id: Option<u64>
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
pub trait LocalResourceRepository: Repository<LocalResource, LocalResourceId> {
    async fn load(&self, path: LocalResourcePath) -> Result<Option<LocalResource>, PersistenceError>;
    async fn save_thumbnail(&self, png_bytes: Vec<u8>, resource_id: u64) -> Result<LocalResourcePath, PersistenceError>;
    async fn get_resource_type(&self, path: LocalResourcePath) -> Result<ResourceType, PersistenceError>;
    async fn load_all(&self) -> Result<Vec<LocalResource>, PersistenceError>;
    async fn read(&self, path: LocalResourcePath, max_length: usize) -> Result<Box<dyn IOReader>, PersistenceError>;
    async fn write(&self, path: LocalResourcePath) -> Result<Box<dyn IOWriter>, PersistenceError>;
    async fn size(&self, path: LocalResourcePath) -> Result<u64, PersistenceError>;
    async fn generate_thumbnail_paths(&self, session_id: Option<u64>, resource_ids: Vec<u64>) -> Result<HashMap<u64, LocalResourcePath>, PersistenceError>;
}

impl Table<LocalResourceId> for LocalResource {
    fn get_table() -> &'static str {
        "localResource"
    }

    fn id(&self) -> LocalResourceId {
        LocalResourceId {
            r#type: Some(self.r#type.clone()),
            path: Some(self.path.as_string()),
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
