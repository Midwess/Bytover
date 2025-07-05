use core_services::db::surrealdb::connection::SurrealDbConnection;
use core_services::utils::pool::request::PoolRequest;
use devlog_sdk::distributed_id::init_id_generator;
use devlog_sdk::grpc_gateway::channel::GrpcGatewayChannel;
use devlog_sdk::sdk::{DependenciesInjection, DevlogSdk};
use schema::devlog::auth_gateway::rpc::auth_service_client::AuthServiceClient;
use schema::devlog::auth_gateway::rpc::user_service_client::UserServiceClient;
use tokio::sync::OnceCell;
use tonic::transport::Channel;

use crate::cloud_storage::storage::CloudStorage;
use crate::grpc::cloud_service::CloudGrpcService;
use crate::grpc::middlewares::auth::AuthInterceptor;
use crate::infrastructure::s3::cloud_storage::S3CloudStorageImpl;
use crate::infrastructure::surrealdb::transfer_session::TransferSessionSurrealdbRepository;
use crate::repositories::transfer_session::TransferSessionRepository;
use crate::transfer::transfer_service::TransferService;

#[derive(Debug, thiserror::Error)]
pub enum DiContainerError {
    #[error("Grpc gateway channel error")]
    GrpcGatewayChannelError(#[from] tonic::transport::Error)
}

static DI_CONTAINER: OnceCell<DiContainer> = OnceCell::const_new();

pub struct DiContainer {
    pub grpc_gateway_channel: GrpcGatewayChannel,
    pub devlog_sdk: DevlogSdk
}

impl DiContainer {
    pub async fn new() -> Self {
        let devlog_sdk = DevlogSdk::new();
        devlog_sdk.enable_system_db().await;
        devlog_sdk.enable_db("bitbridge".to_owned(), 10, 250).await;

        init_id_generator("bitbridge".to_owned(), devlog_sdk.system_db().await).await;

        Self {
            grpc_gateway_channel: GrpcGatewayChannel::new(),
            devlog_sdk
        }
    }

    pub async fn instance() -> &'static DiContainer {
        let instance = DI_CONTAINER.get_or_init(|| async { Self::new().await }).await;

        instance
    }

    pub async fn db(&self) -> PoolRequest<SurrealDbConnection> {
        self.devlog_sdk.db("bitbridge".to_owned()).await
    }

    pub fn get_grpc_gateway_channel(&self) -> &GrpcGatewayChannel {
        &self.grpc_gateway_channel
    }

    pub async fn get_auth_service(&self) -> Result<AuthServiceClient<Channel>, DiContainerError> {
        let channel = self.get_grpc_gateway_channel().connect().await?;

        Ok(AuthServiceClient::new(channel))
    }

    pub async fn get_user_service(&self) -> Result<UserServiceClient<Channel>, DiContainerError> {
        let channel = self.get_grpc_gateway_channel().connect().await?;

        Ok(UserServiceClient::new(channel))
    }

    pub async fn get_transfer_service(&'static self) -> TransferService {
        TransferService {
            transfer_repository: Box::new(self.get_transfer_session_repository().await),
            cloud_storage: Box::new(self.get_cloud_storage())
        }
    }

    pub async fn get_grpc_cloud_service(&'static self) -> CloudGrpcService {
        CloudGrpcService {
            transfer_service: self.get_transfer_service().await,
            cloud_storage: Box::new(self.get_cloud_storage())
        }
    }

    pub fn get_auth_middleware(&'static self) -> AuthInterceptor {
        AuthInterceptor {}
    }

    pub fn get_cloud_storage(&'static self) -> impl CloudStorage {
        S3CloudStorageImpl {
            s3_client: self.devlog_sdk.s3_client()
        }
    }

    pub async fn get_transfer_session_repository(&'static self) -> impl TransferSessionRepository {
        TransferSessionSurrealdbRepository { db: self.db().await }
    }
}
