use crate::config::get_signalling_server_ws_url;
use crate::core_api_impl::bridge::CoreBridgeImpl;
use crate::core_api_impl::net_stream::NetStreamImpl;
use crate::grpc::auth_provider::AuthProvider;
use crate::grpc::auth_server::AuthServer;
use crate::grpc::cloud_server::CloudServer;
use crate::native::executor::NativeExecutor;
use crate::native::p2p::P2PNativeExecutor;
use crate::native::persistent::NativePersistent;
use crate::native::rpc::NativeRpc;
use crate::native::transfer::TransferNative;
use crate::network::cloud::cloud_service::CloudService;
use crate::repository::auth_session::AuthSessionRepositoryImpl;
use crate::repository::local_resource::LocalResourceRepositoryImpl;
use crate::repository::transfer_session::TransferSessionRepositoryImpl;
use crate::repository::RedbPoolProvider;
use crate::ShellRuntime;
use core::panic;
use core_services::utils::pool::allocator::{PoolAllocator, PoolBuilder, PoolResourceProvider};
use core_services::utils::pool::request::PoolRequestBuilder;
use devlog_sdk::distributed_id::init_scoped_id_generator;
use redb::Database;
use shared::app::authentication::service::AuthenticationService;
use shared::app::nearby::nearby_services::NearbyService;
use shared::app::repository::auth_session::AuthSessionRepository;
use shared::app::repository::local_resource::LocalResourceRepository;
use shared::app::repository::path_resolver::PathResolver;
use shared::app::repository::transfer_session::TransferSessionRepository;
use shared::app::transfer::file_selection_service::ResourceTransferSelectionService;
use shared::app::transfer::transfer_service::TransferService;
use shared::core_api::{CoreBridge, NetStream};
use shared::core_transfer_protocol::webrtc::webrtc::WebRtc;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::OnceCell;

static DI_SINGLETON: OnceCell<DiContainer> = OnceCell::const_new();

pub struct DiContainer {
    db: OnceCell<Arc<PoolAllocator<Database>>>,
    path_resolver: OnceCell<Arc<dyn PathResolver>>,
    shell: OnceCell<Arc<dyn ShellRuntime>>,
    core_bridge: OnceCell<Arc<dyn CoreBridge>>,
    native_executor: OnceCell<NativeExecutor>,
    auth_service: OnceCell<AuthenticationService>,
    auth_server: OnceCell<AuthServer>,
    nearby_service: OnceCell<NearbyService>,
    transfer_service: OnceCell<TransferService>,
    transfer_selection_service: OnceCell<ResourceTransferSelectionService>
}

impl DiContainer {
    pub fn get_instance() -> &'static DiContainer {
        DI_SINGLETON.get().unwrap_or_else(|| {
            let instance = DiContainer {
                path_resolver: OnceCell::new(),
                shell: OnceCell::new(),
                core_bridge: OnceCell::new(),
                native_executor: OnceCell::new(),
                db: OnceCell::new(),
                auth_service: OnceCell::new(),
                auth_server: OnceCell::new(),
                nearby_service: OnceCell::new(),
                transfer_service: OnceCell::new(),
                transfer_selection_service: OnceCell::new()
            };

            let _ = DI_SINGLETON.set(instance);
            DI_SINGLETON.get().unwrap()
        })
    }

    pub fn path_resolver(&self) -> &Arc<dyn PathResolver> {
        self.path_resolver.get().unwrap()
    }

    pub fn get_net_stream(&self) -> impl NetStream {
        NetStreamImpl {}
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

    pub async fn init(&self, path_resolver: Arc<dyn PathResolver>, shell: Arc<dyn ShellRuntime>) {
        let _ = self.path_resolver.set(path_resolver);
        let _ = self.shell.set(shell);
        let _ = self.core_bridge.set(Arc::new(CoreBridgeImpl::new(self.shell.get().unwrap().clone())));

        let db_path = self.path_resolver().get_db_path().await;
        log::info!(target: "environment", "Connecting to local database at {db_path}");
        init_scoped_id_generator("BitBridge".to_owned());
        let local_db: Box<dyn PoolResourceProvider<Database>> = Box::new(RedbPoolProvider { path: db_path.clone() });

        let pool = PoolBuilder::new(local_db)
            .max_pool_size(1)
            .min_pool_size(1)
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
            path_resolver: self.path_resolver().clone(),
            db: PoolRequestBuilder::new()
                .retrieving_timeout(Duration::from_secs(30))
                .pool(self.db.get().unwrap().clone())
                .build()
        }
    }

    pub fn get_transfer_session_repository(&self) -> impl TransferSessionRepository {
        TransferSessionRepositoryImpl {
            path_resolver: self.path_resolver().clone(),
            db: PoolRequestBuilder::new()
                .retrieving_timeout(Duration::from_secs(30))
                .pool(self.db.get().unwrap().clone())
                .build()
        }
    }

    pub fn get_native_executor(&'static self) -> &'static NativeExecutor {
        if let Some(executor) = self.native_executor.get() {
            return executor
        }

        let web_rtc = Arc::new(WebRtc::new(
            self.core_bridge.get().unwrap().clone(),
            get_signalling_server_ws_url(),
            Arc::new(self.get_local_resource_repository())
        ));
        let cloud_service = CloudService {
            server: CloudServer::new(self.get_auth_provider()),
            core_bridge: self.core_bridge.get().unwrap().clone(),
            active_session: Default::default(),
            repository: Arc::new(self.get_local_resource_repository()),
            net_stream: Box::new(self.get_net_stream())
        };

        let executor = NativeExecutor {
            rpc: NativeRpc {},
            persistent: NativePersistent {
                auth_session_repository: Box::new(self.get_auth_session_repository()),
                local_resource_repository: Box::new(self.get_local_resource_repository()),
                transfer_session_repository: Box::new(self.get_transfer_session_repository())
            },
            transfer: TransferNative {
                web_rtc: web_rtc.clone(),
                shell_runtime: self.shell.get().unwrap().clone(),
                cloud_service
            },
            p2p: P2PNativeExecutor {
                web_rtc,
                shell_runtime: self.shell.get().unwrap().clone()
            }
        };

        let _ = self.native_executor.set(executor);
        self.native_executor.get().unwrap()
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
