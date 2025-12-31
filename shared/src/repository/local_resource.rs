use crate::entities::local_resource::{LocalResource, LocalResourcePath, ResourceType};
use crate::repository::errors::PersistenceError;
use crate::shell::api::{CIOCursor, DIOWriter};
use core_services::db::repository::abstraction::id::DbId;
use core_services::db::repository::abstraction::repository::Repository;
use core_services::db::repository::abstraction::table::Table;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq)]
pub struct LocalResourceId {
    pub path: Option<LocalResourcePath>,
    pub order_id: Option<u64>
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
pub trait LocalResourceRepository: Repository<LocalResource, LocalResourceId> {
    async fn load(&self, path: LocalResourcePath) -> Result<Option<LocalResource>, PersistenceError>;
    async fn save_thumbnail(&self, png_bytes: Vec<u8>, resource_id: u64) -> Result<LocalResourcePath, PersistenceError>;
    async fn get_resource_type(&self, path: LocalResourcePath) -> Result<ResourceType, PersistenceError>;
    async fn load_all(&self) -> Result<Vec<LocalResource>, PersistenceError>;
    async fn read(
        &self,
        path: LocalResourcePath,
        buffer_size: usize,
        compressed: bool
    ) -> Result<Box<dyn CIOCursor>, PersistenceError>;
    async fn write(&self, path: LocalResourcePath, compressed: bool) -> Result<Box<dyn DIOWriter>, PersistenceError>;
    async fn size(&self, path: LocalResourcePath) -> Result<u64, PersistenceError>;
    async fn generate_thumbnail_paths(
        &self,
        session_id: Option<u64>,
        resource_ids: Vec<u64>
    ) -> Result<HashMap<u64, LocalResourcePath>, PersistenceError>;
    async fn remove(&self, path: LocalResourcePath) -> Result<Vec<LocalResource>, PersistenceError>;
}

impl Table<LocalResourceId> for LocalResource {
    fn get_table() -> &'static str {
        "localResource"
    }

    fn id(&self) -> LocalResourceId {
        LocalResourceId {
            path: Some(self.path.clone()),
            order_id: Some(self.order_id)
        }
    }
}

impl DbId for LocalResourceId {
    type Table = LocalResource;

    fn is_represent(&self, table: &Self::Table) -> bool {
        if let Some(id) = self.order_id {
            if id != table.order_id {
                return false;
            }
        }

        if let Some(path) = &self.path {
            if path != &table.path {
                return false;
            }
        }

        true
    }
}
