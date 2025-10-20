use crate::config::{get_gateway_grpc_url, get_locator_http_url, get_signalling_server_ws_url};
use crate::core_api_impl::net_stream::NetStreamImpl;
use crate::native::executor::NativeExecutor;
use crate::native::p2p::P2PNativeExecutorImpl;
use crate::native::persistent::NativePersistentImpl;
use crate::native::rpc::NativeRpcImpl;
use crate::native::transfer::TransferNativeImpl;
use crate::network::grpc::RpcNetworkModuleImpl;
use crate::repository::auth_session::AuthSessionRepositoryImpl;
use crate::repository::local_resource::LocalResourceRepositoryImpl;
use crate::repository::transfer_session::TransferSessionRepositoryImpl;
use crate::repository::RedbPoolProvider;
use core_services::utils::pool::allocator::{PoolAllocator, PoolBuilder, PoolResourceProvider};
use core_services::utils::pool::request::PoolRequestBuilder;
use devlog_sdk::distributed_id::init_scoped_id_generator;
use redb::Database;
use shared::protocol::public_cloud::cloud_service::CloudService;
use shared::protocol::rpc::auth_provider::AuthProvider;
use shared::protocol::rpc::auth_server::AuthServer;
use shared::protocol::rpc::cloud_server::CloudServer;
use shared::protocol::webrtc::webrtc::WebRtc;
use shared::repository::auth_session::AuthSessionRepository;
use shared::repository::local_resource::LocalResourceRepository;
use shared::repository::path_resolver::PathResolver;
use shared::repository::transfer_session::TransferSessionRepository;
use shared::shell::api::network::InternetConnection;
use shared::shell::api::{CoreBridge, NetStream};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::OnceCell;
use tonic::transport::Channel;

static DI_SINGLETON: OnceCell<DiContainer> = OnceCell::const_new();

pub struct DiContainer {
    db: OnceCell<Arc<PoolAllocator<Database>>>,
    path_resolver: OnceCell<Arc<dyn PathResolver>>,
    core_bridge: OnceCell<&'static dyn CoreBridge>,
    native_executor: OnceCell<NativeExecutor>,
    cloud_server: OnceCell<CloudServer<Channel>>,

    rpc_connection: RpcNetworkModuleImpl
}

impl DiContainer {
    pub fn get_instance() -> &'static DiContainer {
        DI_SINGLETON.get().unwrap_or_else(|| {
            let instance = DiContainer {
                path_resolver: OnceCell::new(),
                core_bridge: OnceCell::new(),
                native_executor: OnceCell::new(),
                db: OnceCell::new(),
                rpc_connection: RpcNetworkModuleImpl::new(get_gateway_grpc_url()),
                cloud_server: OnceCell::new()
            };

            let _ = DI_SINGLETON.set(instance);
            DI_SINGLETON.get().unwrap()
        })
    }

    pub fn path_resolver(&self) -> &Arc<dyn PathResolver> {
        self.path_resolver.get().unwrap()
    }

    pub fn get_net_stream(&'static self, repository: Arc<dyn LocalResourceRepository>) -> impl NetStream {
        NetStreamImpl {
            repository,
            server: self.get_cloud_server()
        }
    }

    pub fn get_authentication_server(&'static self) -> AuthServer<Channel> {
        AuthServer::new(self.get_auth_provider(), Box::new(self.rpc_connection.clone()))
    }

    pub async fn init(&self, path_resolver: Arc<dyn PathResolver>, bridge: &'static dyn CoreBridge) {
        let _ = self.path_resolver.set(path_resolver);
        let _ = self.core_bridge.set(bridge);

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

    pub fn get_cloud_server(&'static self) -> &'static CloudServer<Channel> {
        if let Some(server) = self.cloud_server.get() {
            return server
        };

        let server = CloudServer::new(Box::new(self.rpc_connection.clone()), self.get_auth_provider());
        let _ = self.cloud_server.set(server);
        self.cloud_server.get().unwrap()
    }

    pub fn core_bridge(&'static self) -> &'static dyn CoreBridge {
        *self.core_bridge.get().unwrap()
    }

    pub fn get_native_executor(&'static self) -> &'static NativeExecutor {
        if let Some(executor) = self.native_executor.get() {
            return executor
        }
        let local_resource_repo = Arc::new(self.get_local_resource_repository());
        let web_rtc = Arc::new(WebRtc::new(get_signalling_server_ws_url(), local_resource_repo.clone()));
        let cloud_service = CloudService {
            server: self.get_cloud_server(),
            active_session: Default::default(),
            repository: local_resource_repo.clone(),
            net_stream: Box::new(self.get_net_stream(local_resource_repo.clone()))
        };

        let executor = NativeExecutor {
            internet_connection: InternetConnection::new(get_locator_http_url()),
            rpc: Box::new(NativeRpcImpl {
                auth_server: self.get_authentication_server()
            }),
            persistent: Box::new(NativePersistentImpl {
                auth_session_repository: Box::new(self.get_auth_session_repository()),
                local_resource_repository: Box::new(self.get_local_resource_repository()),
                transfer_session_repository: Box::new(self.get_transfer_session_repository())
            }),
            transfer: Box::new(TransferNativeImpl {
                web_rtc: web_rtc.clone(),
                cloud_service
            }),
            p2p: Box::new(P2PNativeExecutorImpl { web_rtc })
        };

        let _ = self.native_executor.set(executor);
        self.native_executor.get().unwrap()
    }
}
