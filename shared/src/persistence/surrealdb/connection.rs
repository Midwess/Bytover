use core_services::utils::pool::allocator::PoolResourceProvider;
use surrealdb::{engine::{local::{Db, RocksDb}, remote::ws::Client}, Surreal};

#[derive(Debug, Clone)]
pub struct SurrealDbLocalConnectionInfo {
    pub db_path: String,
}

#[derive(Debug, Clone)]
pub struct SurrealDbConnectionProvider {
    pub connection: SurrealDbLocalConnectionInfo,
}

#[async_trait::async_trait]
impl PoolResourceProvider<Surreal<Db>> for SurrealDbConnectionProvider {
    async fn new(&self) -> Surreal<Db> where Self: 'async_trait {
        let conn = Surreal::new::<RocksDb>(self.connection.db_path.clone()).await.unwrap();
        conn
    }
}