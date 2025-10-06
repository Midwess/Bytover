use crate::bridge::bridge::CoreBridgeImpl;
use crate::config::{get_gateway_grpc_url, get_locator_server_url, get_signalling_server_ws_url};
use crate::executor::executor::NativeExecutor;
use crate::executor::p2p::P2PNativeExecutorImpl;
use crate::executor::persistent::NativePersistentImpl;
use crate::executor::rpc::NativeRpcImpl;
use crate::executor::transfer::TransferNativeImpl;
use crate::network::grpc::RpcNetworkModuleImpl;
use crate::network::net_stream::NetStreamImpl;
use crate::repository::auth_session::AuthSessionRepositoryImpl;
use crate::repository::local_resource::LocalResourceRepositoryImpl;
use crate::repository::transfer_session::TransferSessionRepositoryImpl;
use crate::repository::IdbPoolProvider;
use core_services::utils::never_send::NeverSend;
use core_services::utils::pool::allocator::{PoolAllocator, PoolBuilder, PoolResourceProvider};
use core_services::utils::pool::request::PoolRequestBuilder;
use devlog_sdk::distributed_id::init_scoped_id_generator;
use idb::Database;
use once_cell::sync::OnceCell;
use shared::protocol::public_cloud::cloud_service::CloudService;
use shared::protocol::rpc::auth_provider::AuthProvider;
use shared::protocol::rpc::auth_server::AuthServer;
use shared::protocol::rpc::cloud_server::CloudServer;
use shared::protocol::webrtc::webrtc::WebRtc;
use shared::repository::auth_session::AuthSessionRepository;
use shared::repository::local_resource::LocalResourceRepository;
use shared::repository::transfer_session::TransferSessionRepository;
use shared::shell::api::network::InternetConnection;
use shared::shell::api::{CoreBridge, NetStream};
use std::sync::Arc;
use std::time::Duration;
use tonic_web_wasm_client::Client;

static DI_SINGLETON: OnceCell<DiContainer> = OnceCell::new();

pub struct DiContainer {
    db: OnceCell<Arc<PoolAllocator<NeverSend<Database>>>>,
    core_bridge: OnceCell<Arc<dyn CoreBridge>>,
    native_executor: OnceCell<NativeExecutor>,
    cloud_server: OnceCell<CloudServer<Client>>,
    resource_repository: OnceCell<Arc<dyn LocalResourceRepository>>,
    transfer_repository: OnceCell<Arc<dyn TransferSessionRepository>>,

    rpc_connection: RpcNetworkModuleImpl
}

impl DiContainer {
    pub fn get_instance() -> &'static DiContainer {
        DI_SINGLETON.get().unwrap_or_else(|| {
            let instance = DiContainer {
                core_bridge: OnceCell::new(),
                native_executor: OnceCell::new(),
                db: OnceCell::new(),
                rpc_connection: RpcNetworkModuleImpl::new(get_gateway_grpc_url()),
                resource_repository: OnceCell::new(),
                transfer_repository: OnceCell::new(),
                cloud_server: OnceCell::new()
            };

            let _ = DI_SINGLETON.set(instance);
            DI_SINGLETON.get().unwrap()
        })
    }

    pub async fn get_net_stream(&'static self) -> Box<dyn NetStream> {
        Box::new(NetStreamImpl {
            resource_repo: self.get_local_resource_repository().await,
            server: self.get_cloud_server()
        })
    }

    pub fn get_authentication_server(&'static self) -> AuthServer<Client> {
        AuthServer::new(self.get_auth_provider(), Box::new(self.rpc_connection.clone()))
    }

    pub async fn init(&self) {
        let _ = self.core_bridge.set(Arc::new(CoreBridgeImpl::new()));

        init_scoped_id_generator("BitBridge".to_owned());
        let local_db: Box<dyn PoolResourceProvider<NeverSend<Database>>> = Box::new(IdbPoolProvider { name: "db".to_owned() });

        let pool = PoolBuilder::new(local_db)
            .max_pool_size(5)
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

    pub async fn get_local_resource_repository(&self) -> Arc<dyn LocalResourceRepository> {
        if let Some(repository) = self.resource_repository.get() {
            return repository.clone();
        }

        let repo = Arc::new(LocalResourceRepositoryImpl {
            db: PoolRequestBuilder::new()
                .retrieving_timeout(Duration::from_secs(30))
                .pool(self.db.get().unwrap().clone())
                .build()
        });

        let _ = self.resource_repository.set(repo.clone());
        repo
    }

    pub fn get_transfer_session_repository(&self) -> Arc<dyn TransferSessionRepository> {
        if let Some(repository) = self.transfer_repository.get() {
            return repository.clone();
        }

        let repo = Arc::new(TransferSessionRepositoryImpl {
            db: PoolRequestBuilder::new()
                .retrieving_timeout(Duration::from_secs(30))
                .pool(self.db.get().unwrap().clone())
                .build()
        });

        let _ = self.transfer_repository.set(repo.clone());
        repo
    }

    pub fn get_cloud_server(&'static self) -> &'static CloudServer<Client> {
        if let Some(server) = self.cloud_server.get() {
            return server;
        }

        let server = CloudServer::new(Box::new(self.rpc_connection.clone()), self.get_auth_provider());
        let _ = self.cloud_server.set(server);
        self.cloud_server.get().unwrap()
    }

    pub async fn get_native_executor(&'static self) -> &'static NativeExecutor {
        if let Some(executor) = self.native_executor.get() {
            return executor
        }

        let web_rtc = Arc::new(WebRtc::new(
            self.core_bridge.get().unwrap().clone(),
            get_signalling_server_ws_url(),
            self.get_local_resource_repository().await
        ));
        let cloud_service = CloudService {
            server: self.get_cloud_server(),
            core_bridge: self.core_bridge.get().unwrap().clone(),
            active_session: Default::default(),
            repository: self.get_local_resource_repository().await,
            net_stream: self.get_net_stream().await
        };

        let executor = NativeExecutor {
            internet_connection: InternetConnection::new(get_locator_server_url()),
            rpc: Box::new(NativeRpcImpl {
                auth_server: self.get_authentication_server()
            }),
            persistent: Box::new(NativePersistentImpl {
                auth_session_repository: Box::new(self.get_auth_session_repository()),
                local_resource_repository: self.get_local_resource_repository().await,
                transfer_session_repository: self.get_transfer_session_repository()
            }),
            transfer: Box::new(TransferNativeImpl {
                web_rtc: web_rtc.clone(),
                cloud_service,
                cloud_server: self.get_cloud_server(),
                auth_server: self.get_authentication_server()
            }),
            p2p: Box::new(P2PNativeExecutorImpl { web_rtc })
        };

        let _ = self.native_executor.set(executor);
        self.native_executor.get().unwrap()
    }
}
