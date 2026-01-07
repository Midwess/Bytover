use crate::entities::shelf::Shelf;
use crate::repository::errors::PersistenceError;
use core_services::db::repository::abstraction::id::DbId;
use core_services::db::repository::abstraction::repository::Repository;
use core_services::db::repository::abstraction::table::Table;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq)]
pub struct ShelfId {
    pub id: Option<u64>
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
pub trait ShelfRepository: Repository<Shelf, ShelfId> {
    async fn load_all(&self) -> Result<Vec<Shelf>, PersistenceError>;
    async fn add(&self, shelf: Shelf) -> Result<Shelf, PersistenceError>;
    async fn remove(&self, id: u64) -> Result<bool, PersistenceError>;
}

impl Table<ShelfId> for Shelf {
    fn get_table() -> &'static str {
        "shelf"
    }

    fn id(&self) -> ShelfId {
        ShelfId { id: Some(self.id) }
    }
}

impl DbId for ShelfId {
    type Table = Shelf;

    fn is_represent(&self, table: &Self::Table) -> bool {
        if let Some(id) = self.id {
            return id == table.id;
        }
        true
    }
}
