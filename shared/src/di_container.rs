use core::panic;
use std::sync::Arc;
use std::time::Duration;

use core_services::utils::pool::allocator::{PoolAllocator, PoolBuilder, PoolResourceProvider};
use core_services::utils::pool::request::PoolRequestBuilder;
use surrealdb::engine::any::Any;
use surrealdb::Surreal;
use tokio::sync::OnceCell;
use tokio_scoped::scoped;

use crate::app::authentication::service::AuthenticationService;
use crate::app::transfer::file_selection_service::ResourceTransferSelectionService;
use crate::app::transfer::nearby::NearbyService;
use crate::grpc::auth_server::AuthServer;
use crate::native::database::NativeDatabase;
use crate::native::executor::NativeExecutor;
use crate::native::local_storage::NativeLocalStorage;
use crate::native::rpc::NativeRpc;
use crate::native::transfer::TransferNative;
use crate::network::webrtc::web_rtc::WebRtc;
use crate::persistence::local_resource::LocalResourceRepository;
use crate::persistence::session::SessionRepository;
use crate::persistence::surrealdb::connection::{SurrealDbConnectionProvider, SurrealDbLocalConnectionInfo};
use crate::{get_tokio_rt, ShellRuntime};

static DI_SINGLETON: OnceCell<DiContainer> = OnceCell::const_new();

pub struct DiContainer {
    db: OnceCell<Arc<PoolAllocator<Surreal<Any>>>>,
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

    pub fn init(&self, work_dir_path: String) {
        scoped(get_tokio_rt().handle()).scope(move |scope| {
            scope.spawn(async move {
                let db_path = format!("{}/{}", work_dir_path, "surrealdb.db");
                log::info!(target: "environment", "Connecting to local database at {}", db_path);
                let local_db: Box<dyn PoolResourceProvider<Surreal<Any>>> = Box::new(SurrealDbConnectionProvider {
                    connection: SurrealDbLocalConnectionInfo { db_path: db_path.clone() }
                });

                let _ = self.db.set(
                    PoolBuilder::new(local_db)
                        .max_pool_size(1)
                        .min_pool_size(0)
                        .resource_idle_timeout(Duration::from_secs(5))
                        .build()
                        .await
                );

                log::info!(target: "native", "Initializing authentication server");
                let server = AuthServer::new(self.get_session_repository()).await;
                let _ = self.auth_server.set(server);
            });
        });
    }

    pub fn get_session_repository(&self) -> SessionRepository {
        SessionRepository {
            db: PoolRequestBuilder::new()
                .retrieving_timeout(Duration::from_secs(30))
                .pool(self.db.get().unwrap().clone())
                .build()
        }
    }

    pub fn get_local_resource_repository(&self) -> LocalResourceRepository {
        LocalResourceRepository {
            db: PoolRequestBuilder::new()
                .retrieving_timeout(Duration::from_secs(30))
                .pool(self.db.get().unwrap().clone())
                .build()
        }
    }

    pub fn get_native_executor(&self, shell_runtime: Arc<dyn ShellRuntime>) -> NativeExecutor {
        NativeExecutor {
            rpc: NativeRpc {},
            database: NativeDatabase {
                session_repository: self.get_session_repository(),
                local_resource_repository: self.get_local_resource_repository()
            },
            local_storage: NativeLocalStorage {},
            transfer: TransferNative {
                web_rtc: Arc::new(WebRtc::new()),
                shell_runtime: OnceCell::new()
            }
        }
    }

    pub fn get_resource_transfer_selection_service(&self) -> ResourceTransferSelectionService {
        ResourceTransferSelectionService {}
    }

    pub fn get_nearby_service(&self) -> NearbyService {
        NearbyService {}
    }
}
