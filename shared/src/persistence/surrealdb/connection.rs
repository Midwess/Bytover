use core_services::utils::pool::allocator::PoolResourceProvider;
use surrealdb::engine::any::{self, Any};
use surrealdb::Surreal;

#[derive(Debug, Clone)]
pub struct SurrealDbLocalConnectionInfo {
    pub db_path: String
}

#[derive(Debug, Clone)]
pub struct SurrealDbConnectionProvider {
    pub connection: SurrealDbLocalConnectionInfo
}

#[async_trait::async_trait]
impl PoolResourceProvider<Surreal<Any>> for SurrealDbConnectionProvider {
    async fn new(&self) -> Surreal<Any>
    where
        Self: 'async_trait
    {
        let surreal_kv_path = format!("surrealkv://{}", self.connection.db_path);
        let conn = any::connect(surreal_kv_path).with_capacity(1).await.unwrap();
        conn.use_ns("main").use_db("main").await.unwrap();
        conn
    }
}
