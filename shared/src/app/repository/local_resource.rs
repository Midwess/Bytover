use crate::app::file_system::file::{LocalResource, LocalResourcePath, ResourceType};
use core_services::db::repository::abstraction::id::DbId;
use core_services::db::repository::abstraction::repository::Repository;
use core_services::db::repository::abstraction::table::Table;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct LocalResourceId {
    pub r#type: Option<ResourceType>,
    pub path: Option<LocalResourcePath>,
    pub order_id: Option<u64>
}

#[async_trait::async_trait]
pub trait LocalResourceRepository: Repository<LocalResource, LocalResourceId> {}

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
