use crate::app_gateway::app_info::AppInfoService;
use crate::app_gateway::markov::Markov;
use crate::cloud_storage::storage::CloudStorage;
use crate::grpc::cloud_service::CloudGrpcService;
use crate::grpc::middlewares::auth::AuthInterceptor;
use crate::infrastructure::app_gateway::AppGatewayImpl;
use crate::infrastructure::mail::email_service::EmailServiceImpl;
use crate::infrastructure::s3::cloud_storage::S3CloudStorageImpl;
use crate::infrastructure::surrealdb::transfer_session::TransferSessionSurrealdbRepository;
use crate::mail::service::EmailService;
use crate::repositories::transfer_session::TransferSessionRepository;
use crate::transfer::transfer_service::TransferService;
use crate::user::Token;
use core_services::db::surrealdb::connection::SurrealDbConnection;
use core_services::utils::pool::request::PoolRequest;
use devlog_sdk::distributed_id::init_id_generator;
use devlog_sdk::grpc_gateway::channel::GrpcGatewayChannel;
use devlog_sdk::live_query::live_query::LiveQuery;
use devlog_sdk::sdk::{DependenciesInjection, DevlogSdk};
use schema::devlog::auth_gateway::rpc::auth_service_client::AuthServiceClient;
use schema::devlog::auth_gateway::rpc::mail_service_client::MailServiceClient;
use schema::devlog::auth_gateway::rpc::user_service_client::UserServiceClient;
use std::sync::Arc;
use tokio::sync::OnceCell;
use tonic::transport::Channel;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

#[derive(Debug, thiserror::Error)]
pub enum DiContainerError {
    #[error("Grpc gateway channel error")]
    GrpcGatewayChannelError(#[from] tonic::transport::Error)
}

static DI_CONTAINER: OnceCell<DiContainer> = OnceCell::const_new();

pub struct DiContainer {
    pub grpc_gateway_channel: GrpcGatewayChannel,
    pub devlog_sdk: DevlogSdk,
    live_query: Arc<LiveQuery>,
    pg_pool: PgPool
}

impl DiContainer {
    pub async fn new() -> Self {
        let devlog_sdk = DevlogSdk::new();
        devlog_sdk.enable_system_db().await;
        devlog_sdk.enable_db("bitbridge".to_owned(), 2, 256).await;

        init_id_generator("bitbridge".to_owned(), devlog_sdk.system_db().await).await;

        let app_db = devlog_sdk.db("bitbridge".to_owned()).await;

        let database_url = std::env::var("BITBRIDGE_DB_CONNECTION_STRING").expect("BITBRIDGE_DB_CONNECTION_STRING must be defined.");
        let pg_pool = PgPoolOptions::new()
            .min_connections(5)
            .max_connections(10)
            .connect(&database_url)
            .await
            .expect("Failed to connect to Postgres using sqlx.");

        Self {
            grpc_gateway_channel: GrpcGatewayChannel::new(),
            devlog_sdk,
            live_query: Arc::new(LiveQuery::new(app_db).await),
            pg_pool
        }
    }

    pub async fn instance() -> &'static DiContainer {
        let instance = DI_CONTAINER.get_or_init(|| async { Self::new().await }).await;

        instance
    }

    pub fn get_pg_pool(&'static self) -> &'static PgPool {
        &self.pg_pool
    }

    pub async fn db(&self) -> PoolRequest<SurrealDbConnection> {
        self.devlog_sdk.db("bitbridge".to_owned()).await
    }

    pub fn markov_generator(&self) -> impl Markov {
        AppGatewayImpl {
            channel: self.grpc_gateway_channel.clone()
        }
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

    pub async fn get_mail_service(&self) -> Result<MailServiceClient<Channel>, DiContainerError> {
        let channel = self.get_grpc_gateway_channel().connect().await?;

        Ok(MailServiceClient::new(channel))
    }

    pub async fn get_transfer_service(&'static self, token: Token) -> TransferService {
        TransferService {
            transfer_repository: Box::new(self.get_transfer_session_repository().await),
            cloud_storage: Box::new(self.get_cloud_storage()),
            markov_generator: Box::new(self.markov_generator()),
            email_service: Box::new(self.get_email_service(token).await.unwrap()),
            app_service: Box::new(self.get_app_service().await)
        }
    }

    pub async fn get_grpc_cloud_service(&'static self) -> CloudGrpcService {
        CloudGrpcService {
            cloud_storage: Arc::new(self.get_cloud_storage()),
            live_query: self.live_query.clone(),
            session_repository: Box::new(self.get_transfer_session_repository().await),
            app_service: Box::new(self.get_app_service().await)
        }
    }

    pub async fn get_app_service(&'static self) -> impl AppInfoService {
        AppGatewayImpl {
            channel: self.grpc_gateway_channel.clone()
        }
    }

    pub fn get_auth_middleware(&'static self) -> AuthInterceptor {
        AuthInterceptor {}
    }

    pub fn get_cloud_storage(&'static self) -> impl CloudStorage {
        S3CloudStorageImpl {
            s3_client: self.devlog_sdk.s3_client(),
            cached_sign: Arc::new(Default::default())
        }
    }

    pub async fn get_transfer_session_repository(&'static self) -> impl TransferSessionRepository {
        TransferSessionSurrealdbRepository { db: self.db().await }
    }

    pub async fn get_email_service(&'static self, token: Token) -> Result<impl EmailService, DiContainerError> {
        let mail_service = self.get_mail_service().await?;

        Ok(EmailServiceImpl::new(mail_service, Some(token)))
    }
}
