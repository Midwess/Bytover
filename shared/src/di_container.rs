use core::panic;
use std::sync::Arc;
use std::time::Duration;

use core_services::utils::pool::allocator::{PoolAllocator, PoolBuilder, PoolResourceProvider};
use core_services::utils::pool::request::PoolRequestBuilder;
use surrealdb::engine::local::Db;
use surrealdb::Surreal;
use tokio::sync::OnceCell;
use tokio_scoped::scoped;

use crate::app::authentication::service::AuthenticationService;
use crate::app::transfer::file_selection_service::ResourceTransferSelectionService;
use crate::app::transfer::service::TransferService;
use crate::grpc::auth_server::AuthServer;
use crate::native::database::NativeDatabase;
use crate::native::executor::NativeExecutor;
use crate::native::local_storage::NativeLocalStorage;
use crate::native::rpc::NativeRpc;
use crate::persistence::session::SessionRepository;
use crate::persistence::surrealdb::connection::{SurrealDbConnectionProvider, SurrealDbLocalConnectionInfo};
use crate::persistence::transfer_session::TransferSessionRepository;
use crate::TOKIO_RT;

static DI_SINGLETON: OnceCell<DiContainer> = OnceCell::const_new();

pub struct DiContainer {
    db: OnceCell<Arc<PoolAllocator<Surreal<Db>>>>,
    auth_service: OnceCell<AuthenticationService>,
    auth_server: OnceCell<AuthServer>
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

    pub fn get_authentication_service(&'static self) -> &'static AuthenticationService {
        match self.auth_service.get() {
            Some(service) => service,
            None => {
                let service = AuthenticationService {};

                let _ = self.auth_service.set(service);
                self.auth_service.get().unwrap()
            }
        }
    }

    pub fn get_authentication_server(&'static self) -> &'static AuthServer {
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
                        db_path: work_dir_path.clone()
                    }
                });

                let _ = self.db.set(
                    PoolBuilder::new(local_db)
                        .max_pool_size(1)
                        .min_pool_size(1)
                        .resource_idle_timeout(Duration::from_secs(10))
                        .build()
                        .await
                );

                let server = AuthServer::new(self.get_session_repository()).await;
                let _ = self.auth_server.set(server);
            });
        });
    }

    pub fn get_session_repository(&self) -> SessionRepository {
        SessionRepository {
            db: PoolRequestBuilder::new()
                .retrieving_timeout(Duration::from_secs(10))
                .pool(self.db.get().unwrap().clone())
                .build()
        }
    }

    pub fn get_transfer_session_repository(&self) -> TransferSessionRepository {
        TransferSessionRepository {
            db: PoolRequestBuilder::new()
                .retrieving_timeout(Duration::from_secs(10))
                .pool(self.db.get().unwrap().clone())
                .build()
        }
    }

    pub async fn get_native_executor(&self) -> NativeExecutor {
        NativeExecutor {
            rpc: NativeRpc {},
            database: NativeDatabase {
                session_repository: self.get_session_repository(),
                transfer_session_repository: self.get_transfer_session_repository()
            },
            local_storage: NativeLocalStorage {}
        }
    }

    pub fn get_transfer_service(&self) -> TransferService {
        TransferService {}
    }

    pub fn get_resource_transfer_selection_service(&self) -> ResourceTransferSelectionService {
        ResourceTransferSelectionService {}
    }
}
