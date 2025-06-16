use core_services::db::remote_surrealdb::SurrealDbConnection;
use core_services::utils::pool::request::PoolRequest;
use devlog_sdk::grpc_gateway::channel::GrpcGatewayChannel;
use devlog_sdk::sdk::{DependenciesInjection, DevlogSdk};
use schema::devlog::auth_gateway::rpc::auth_service_client::AuthServiceClient;
use schema::devlog::auth_gateway::rpc::user_service_client::UserServiceClient;
use tokio::sync::OnceCell;
use tonic::transport::Channel;

use crate::infrastructure::surrealdb::transfer_session::TransferSessionSurrealdbRepository;
use crate::repositories::transfer_session::TransferSessionRepository;

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
        devlog_sdk.enable_db("bitbridge".to_owned(), 0, 250).await;

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

    pub async fn get_transfer_session_repository(&'static self) -> impl TransferSessionRepository {
        TransferSessionSurrealdbRepository { db: self.db().await }
    }
}
