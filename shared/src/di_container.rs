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
use crate::app::file_system::workdir::WorkDir;
use crate::app::nearby::nearby_services::NearbyService;
use crate::app::transfer::file_selection_service::ResourceTransferSelectionService;
use crate::app::transfer::transfer_service::TransferService;
use crate::get_tokio_rt;
use crate::grpc::auth_server::AuthServer;
use crate::native::database::NativeDatabase;
use crate::native::executor::NativeExecutor;
use crate::native::local_storage::NativeLocalStorage;
use crate::native::p2p::P2PNativeExecutor;
use crate::native::rpc::NativeRpc;
use crate::native::transfer::TransferNative;
use crate::network::webrtc::web_rtc::WebRtc;
use crate::persistence::local_resource::LocalResourceRepository;
use crate::persistence::session::SessionRepository;
use crate::persistence::surrealdb::connection::{SurrealDbConnectionProvider, SurrealDbLocalConnectionInfo};
use crate::persistence::transfer_session::TransferSessionRepository;

static DI_SINGLETON: OnceCell<DiContainer> = OnceCell::const_new();

pub struct DiContainer {
    db: OnceCell<Arc<PoolAllocator<Surreal<Any>>>>,
    auth_service: OnceCell<AuthenticationService>,
    auth_server: OnceCell<AuthServer>,
    workdir: OnceCell<WorkDir>,
    nearby_service: OnceCell<NearbyService>,
    transfer_service: OnceCell<TransferService>,
    transfer_selection_service: OnceCell<ResourceTransferSelectionService>
}

impl DiContainer {
    pub fn get_instance() -> &'static DiContainer {
        match DI_SINGLETON.get() {
            Some(instance) => instance,
            None => {
                let instance = DiContainer {
                    db: OnceCell::new(),
                    auth_service: OnceCell::new(),
                    auth_server: OnceCell::new(),
                    workdir: OnceCell::new(),
                    nearby_service: OnceCell::new(),
                    transfer_service: OnceCell::new(),
                    transfer_selection_service: OnceCell::new()
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

    pub fn get_transfer_service(&'static self) -> &'static TransferService {
        match self.transfer_service.get() {
            Some(service) => service,
            None => {
                let service = TransferService {};
                let _ = self.transfer_service.set(service);
                self.transfer_service.get().unwrap()
            }
        }
    }

    pub fn init(&self, work_dir: WorkDir) {
        let _ = self.workdir.set(work_dir.clone());
        scoped(get_tokio_rt().handle()).scope(move |scope| {
            scope.spawn(async move {
                let db_path = work_dir.database();
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

    pub fn get_transfer_session_repository(&self) -> TransferSessionRepository {
        TransferSessionRepository {
            db: PoolRequestBuilder::new()
                .retrieving_timeout(Duration::from_secs(30))
                .pool(self.db.get().unwrap().clone())
                .build()
        }
    }

    pub fn get_native_executor(&self) -> NativeExecutor {
        let web_rtc = Arc::new(WebRtc::new(self.workdir.get().unwrap().clone()));
        NativeExecutor {
            rpc: NativeRpc {},
            database: NativeDatabase {
                session_repository: self.get_session_repository(),
                local_resource_repository: self.get_local_resource_repository(),
                transfer_session_repository: self.get_transfer_session_repository()
            },
            local_storage: NativeLocalStorage {},
            transfer: TransferNative {
                web_rtc: web_rtc.clone(),
                shell_runtime: OnceCell::new()
            },
            p2p: P2PNativeExecutor {
                web_rtc,
                shell_runtime: OnceCell::new()
            }
        }
    }

    pub fn get_resource_transfer_selection_service(&'static self) -> &'static ResourceTransferSelectionService {
        match self.transfer_selection_service.get() {
            Some(service) => service,
            None => {
                let service = ResourceTransferSelectionService {};
                let _ = self.transfer_selection_service.set(service);
                self.transfer_selection_service.get().unwrap()
            }
        }
    }

    pub fn get_nearby_service(&'static self) -> &'static NearbyService {
        match self.nearby_service.get() {
            Some(service) => service,
            None => {
                let service = NearbyService {};
                let _ = self.nearby_service.set(service);
                self.nearby_service.get().unwrap()
            }
        }
    }
}
