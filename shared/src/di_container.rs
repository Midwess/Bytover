use core::panic;
use std::{sync::Arc, time::Duration};

use core_services::utils::pool::allocator::{PoolAllocator, PoolBuilder, PoolResourceProvider};
use surrealdb::{engine::local::Db, Surreal};
use tokio::sync::OnceCell;
use tokio_scoped::scoped;

use crate::{app::{authentication::service::AuthenticationService, modules::{authentication::AuthenticationModule, environment::EnvironmentModule}, ports::authentication_service::AuthenticationServer}, grpc::auth_server::AuthServer, persistence::surrealdb::connection::{SurrealDbConnectionProvider, SurrealDbLocalConnectionInfo}, TOKIO_RT};

static DI_SINGLETON: OnceCell<DiContainer> = OnceCell::const_new();

pub struct DiContainer {
    db: OnceCell<Arc<PoolAllocator<Surreal<Db>>>>,
    auth_service: OnceCell<AuthenticationService>,
    auth_server: OnceCell<Box<dyn AuthenticationServer>>
}

impl DiContainer {
    pub fn get_instance() -> &'static DiContainer {
        match DI_SINGLETON.get() {
            Some(instance) => instance,
            None => {
                let instance = DiContainer {
                    db: OnceCell::new(),
                    auth_service: OnceCell::new(),
                    auth_server: OnceCell::new()
                };

                let _ = DI_SINGLETON.set(instance);
                DI_SINGLETON.get().unwrap()
            }
        }
    }

    pub fn get_environment_module(&'static self) -> EnvironmentModule {
        EnvironmentModule {}
    }

    pub fn get_authentication_module(&'static self) -> AuthenticationModule {
        AuthenticationModule {
            auth_service: self.get_authentication_service()
        }
    }

    pub fn get_authentication_service(&'static self) -> &'static AuthenticationService {
        match self.auth_service.get() {
            Some(service) => service,
            None => {
                let service = AuthenticationService {
                    auth_server: self.get_authentication_server()
                };

                let _ = self.auth_service.set(service);
                self.auth_service.get().unwrap()
            }
        }
    }

    pub fn get_authentication_server(&'static self) -> &'static Box<dyn AuthenticationServer> {
        match self.auth_server.get() {
            Some(server) => server,
            None => {
                panic!("Authentication server not initialized");
            }
        }
    }

    pub async fn init(&self, work_dir_path: String) {
        scoped(TOKIO_RT.handle()).scope(|scope| {
            scope.spawn(async move {
                let local_db: Box<dyn PoolResourceProvider<Surreal<Db>>> = Box::new(SurrealDbConnectionProvider {
                    connection: SurrealDbLocalConnectionInfo {
                        db_path: work_dir_path.clone(),
                    },
                });

                let _ = self.db.set(PoolBuilder::new(local_db)
                    .max_pool_size(1)
                    .min_pool_size(1)
                    .resource_idle_timeout(Duration::from_secs(10))
                    .build().await
                );

                let server = AuthServer::new().await;
                let _ = self.auth_server.set(Box::new(server));
            });
        });
    }
}
