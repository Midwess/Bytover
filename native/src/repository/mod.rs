use async_trait::async_trait;
use core_services::utils::pool::allocator::PoolResourceProvider;
use redb::Database;

pub mod auth_session;
pub mod id;
pub mod local_resource;
pub mod path_resolver;
pub mod transfer_session;
pub mod user;

pub struct RedbPoolProvider {
    pub path: String
}

#[async_trait]
impl PoolResourceProvider<Database> for RedbPoolProvider {
    async fn new(&self) -> Database
    where
        Self: 'async_trait
    {
        Database::create(self.path.clone()).unwrap()
    }
}
