use crate::get_tokio_rt;
use crate::grpc::auth_provider::AuthProvider;
use crate::grpc::auth_server::AuthServer;
use crate::grpc::cloud_server::CloudServer;
use crate::native::database::NativeDatabase;
use crate::native::executor::NativeExecutor;
use crate::native::local_storage::NativeLocalStorage;
use crate::native::p2p::P2PNativeExecutor;
use crate::native::rpc::NativeRpc;
use crate::native::transfer::TransferNative;
use crate::network::cloud::cloud_service::CloudService;
use crate::network::webrtc::web_rtc::WebRtc;
use core::panic;
use core_services::utils::pool::allocator::{PoolAllocator, PoolBuilder, PoolResourceProvider};
use core_services::utils::pool::request::PoolRequestBuilder;
use redb::Database;
use shared::app::authentication::service::AuthenticationService;
use shared::app::file_system::workdir::WorkDir;
use shared::app::nearby::nearby_services::NearbyService;
use shared::app::transfer::file_selection_service::ResourceTransferSelectionService;
use shared::app::transfer::transfer_service::TransferService;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::OnceCell;

use crate::repository::auth_session::AuthSessionRepositoryImpl;
use crate::repository::local_resource::LocalResourceRepositoryImpl;
use crate::repository::transfer_session::TransferSessionRepositoryImpl;
use crate::repository::RedbPoolProvider;
use shared::app::repository::auth_session::AuthSessionRepository;
use shared::app::repository::local_resource::LocalResourceRepository;
use shared::app::repository::transfer_session::TransferSessionRepository;

static DI_SINGLETON: OnceCell<DiContainer> = OnceCell::const_new();

pub struct DiContainer {
    db: OnceCell<Arc<PoolAllocator<Database>>>,
    auth_service: OnceCell<AuthenticationService>,
    auth_server: OnceCell<AuthServer>,
    workdir: OnceCell<WorkDir>,
    nearby_service: OnceCell<NearbyService>,
    transfer_service: OnceCell<TransferService>,
    transfer_selection_service: OnceCell<ResourceTransferSelectionService>
}

impl DiContainer {
    pub fn get_instance() -> &'static DiContainer {
        DI_SINGLETON.get().unwrap_or_else(|| {
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
        })
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

    pub async fn init(&self, work_dir: WorkDir) {
        let _ = self.workdir.set(work_dir.clone());
        let db_path = work_dir.database();
        log::info!(target: "environment", "Connecting to local database at {}", db_path);
        let local_db: Box<dyn PoolResourceProvider<Database>> = Box::new(RedbPoolProvider { path: db_path.clone() });

        let pool = PoolBuilder::new(local_db)
            .max_pool_size(1)
            .min_pool_size(0)
            .resource_idle_timeout(Duration::from_secs(5))
            .build()
            .await;

        let _ = self.db.set(pool);

        log::info!(target: "native", "Initializing authentication server");
        let server = AuthServer::new(self.get_auth_provider()).await;
        let _ = self.auth_server.set(server);
    }

    pub fn get_auth_provider(&self) -> AuthProvider {
        AuthProvider {
            session_repository: Box::new(self.get_auth_session_repository())
        }
    }

    pub fn get_auth_session_repository(&self) -> impl AuthSessionRepository {
        AuthSessionRepositoryImpl {
            db: PoolRequestBuilder::new()
                .retrieving_timeout(Duration::from_secs(30))
                .pool(self.db.get().unwrap().clone())
                .build()
        }
    }

    pub fn get_local_resource_repository(&self) -> impl LocalResourceRepository {
        LocalResourceRepositoryImpl {
            db: PoolRequestBuilder::new()
                .retrieving_timeout(Duration::from_secs(30))
                .pool(self.db.get().unwrap().clone())
                .build()
        }
    }

    pub fn get_transfer_session_repository(&self) -> impl TransferSessionRepository {
        TransferSessionRepositoryImpl {
            db: PoolRequestBuilder::new()
                .retrieving_timeout(Duration::from_secs(30))
                .pool(self.db.get().unwrap().clone())
                .build()
        }
    }

    pub fn get_native_executor(&self) -> NativeExecutor {
        let web_rtc = Arc::new(WebRtc::new(self.workdir.get().unwrap().clone()));
        let cloud_service = CloudService::new(CloudServer::new(self.get_auth_provider()));

        NativeExecutor {
            rpc: NativeRpc {},
            database: NativeDatabase {
                auth_session_repository: Box::new(self.get_auth_session_repository()),
                local_resource_repository: Box::new(self.get_local_resource_repository()),
                transfer_session_repository: Box::new(self.get_transfer_session_repository())
            },
            local_storage: NativeLocalStorage {
                workdir: self.workdir.get().unwrap().clone()
            },
            transfer: TransferNative {
                web_rtc: web_rtc.clone(),
                shell_runtime: OnceCell::new(),
                cloud_service
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
