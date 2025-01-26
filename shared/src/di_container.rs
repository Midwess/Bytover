use std::sync::Arc;

use core_services::utils::pool::allocator::{PoolAllocator, PoolBuilder, PoolResourceProvider};
use surrealdb::{engine::local::Db, Surreal};
use tokio::sync::OnceCell;

use crate::persistence::surrealdb::connection::{SurrealDbConnectionProvider, SurrealDbLocalConnectionInfo};

static DI_SINGLETON: OnceCell<DiContainer> = OnceCell::const_new();

pub struct DiContainer {
    local_db: OnceCell<Arc<PoolAllocator<Surreal<Db>>>>
}

impl DiContainer {
    pub async fn get_instance() -> &'static DiContainer {
        match DI_SINGLETON.get() {
            Some(instance) => instance,
            None => {
                let instance = DiContainer {
                    local_db: OnceCell::new(),
                };

                instance.setup_local_db().await;

                let _ = DI_SINGLETON.set(instance);
                DI_SINGLETON.get().unwrap()
            }
        }
    }

    async fn setup_local_db(&self) {
        let local_db : Box<dyn PoolResourceProvider<Surreal<Db>>> = Box::new(SurrealDbConnectionProvider {
            connection: SurrealDbLocalConnectionInfo {
                db_path: "data/surrealdb".to_string(),
            },
        });

        self.local_db.set(PoolBuilder::new(local_db)
            .max_pool_size(1)
            .min_pool_size(1)
            .build().await
        );
    }
}