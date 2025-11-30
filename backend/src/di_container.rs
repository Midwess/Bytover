use crate::app_gateway::app_info::AppInfoService;
use crate::app_gateway::markov::Markov;
use crate::cloud_storage::storage::CloudStorage;
use crate::grpc::cloud_service::CloudGrpcService;
use crate::grpc::middlewares::auth::AuthInterceptor;
use crate::infrastructure::app_gateway::AppGatewayImpl;
use crate::infrastructure::mail::email_service::EmailServiceImpl;
use crate::infrastructure::postgres::transfer_session::TransferSessionPostgresRepository;
use crate::infrastructure::s3::cloud_storage::S3CloudStorageImpl;
use crate::mail::service::EmailService;
use crate::repositories::transfer_session::TransferSessionRepository;
use crate::transfer::transfer_service::TransferService;
use crate::user::Token;
use devlog_sdk::distributed_id::{init_id_generator, EtcdWorkerOptions};
use devlog_sdk::grpc_gateway::channel::GrpcGatewayChannel;
use devlog_sdk::sdk::{DependenciesInjection, DevlogSdk};
use migration::{Migrator, MigratorTrait};
use schema::devlog::app_gateway::rpc::auth_service_client::AuthServiceClient;
use schema::devlog::app_gateway::rpc::mail_service_client::MailServiceClient;
use schema::devlog::app_gateway::rpc::user_service_client::UserServiceClient;
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::OnceCell;
use tonic::transport::Channel;
use tokio_cron_scheduler::{Job, JobScheduler};

#[derive(Debug, thiserror::Error)]
pub enum DiContainerError {
    #[error("Grpc gateway channel error")]
    GrpcGatewayChannelError(#[from] tonic::transport::Error),
    #[error("Cron error {0}")]
    CronError(String)
}

static DI_CONTAINER: OnceCell<DiContainer> = OnceCell::const_new();

pub struct DiContainer {
    pub grpc_gateway_channel: GrpcGatewayChannel,
    pub devlog_sdk: DevlogSdk,
    db_connection: DatabaseConnection,
    pub pg_pool: PgPool
}

impl DiContainer {
    pub async fn new() -> Self {
        let devlog_sdk = DevlogSdk::new();

        let etcd_endpoints = std::env::var("SNOWFLAKE_ETCD_ENDPOINTS")
            .unwrap_or_else(|_| "http://localhost:2379".to_string());
        let endpoints: Vec<String> = etcd_endpoints
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        
        let mut etcd_options = EtcdWorkerOptions::new(endpoints);
        
        let namespace = std::env::var("SNOWFLAKE_ETCD_NAMESPACE")
            .unwrap_or_else(|_| "dev".to_string());
        if !namespace.trim().is_empty() {
            etcd_options = etcd_options.namespace(namespace);
        }
        
        let ttl_secs = std::env::var("SNOWFLAKE_ETCD_LEASE_TTL_SECS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(30);
        etcd_options = etcd_options.lease_ttl(std::time::Duration::from_secs(ttl_secs.max(1)));
        
        init_id_generator("bytover".to_owned(), etcd_options)
            .await
            .unwrap_or_else(|err| panic!("Failed to initialise distributed id generator: {err}"));

        let database_url = std::env::var("BYTOVER_DB_CONNECTION_STRING").unwrap_or_else(|_| "postgresql://bitbridge:bitbridgepass@localhost:5432/bitbridge".to_string());
        let pg_pool = PgPoolOptions::new()
            .min_connections(5)
            .max_connections(10)
            .connect(&database_url)
            .await
            .unwrap_or_else(|e| panic!("Failed to create SQLx pool: {e}"));

        let mut opt = ConnectOptions::new(database_url.clone());
        opt.max_connections(20).min_connections(5);
        let db_connection = Database::connect(opt).await.unwrap_or_else(|e| panic!("Failed to connect to Postgres: {e}"));
        Migrator::up(&db_connection, None)
            .await
            .unwrap_or_else(|e| panic!("Failed to run DB migration: {e}"));

        Self {
            grpc_gateway_channel: GrpcGatewayChannel::new(),
            devlog_sdk,
            db_connection,
            pg_pool
        }
    }

    pub async fn instance() -> &'static DiContainer {
        let instance = DI_CONTAINER.get_or_init(|| async { Self::new().await }).await;

        instance
    }

    pub fn get_db_connection(&self) -> DatabaseConnection {
        self.db_connection.clone()
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
            session_repository: Arc::new(self.get_transfer_session_repository().await),
            app_service: Box::new(self.get_app_service().await),
            pg_pool: self.pg_pool.clone()
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
        TransferSessionPostgresRepository { db: self.get_db_connection() }
    }

    pub async fn get_email_service(&'static self, token: Token) -> Result<impl EmailService, DiContainerError> {
        let mail_service = self.get_mail_service().await?;

        Ok(EmailServiceImpl::new(mail_service, Some(token)))
    }

    pub async fn start_cron_jobs(&'static self) -> Result<(), DiContainerError> {
        let sched = JobScheduler::new().await.map_err(|e| DiContainerError::CronError(e.to_string()))?;
        
        let job = Job::new_async("0 */5 * * * *", |_uuid, _l| {
            Box::pin(async move {
                log::info!("Running cleanup cron job...");
                let repo = DiContainer::instance().await.get_transfer_session_repository().await;
                if let Err(e) = repo.delete_expired_or_canceled_sessions().await {
                    log::error!("Failed to cleanup sessions: {}", e);
                }
                log::info!("Cleanup cron job finished.");
            })
        }).map_err(|e| DiContainerError::CronError(e.to_string()))?;

        sched.add(job).await.map_err(|e| DiContainerError::CronError(e.to_string()))?;
        sched.start().await.map_err(|e| DiContainerError::CronError(e.to_string()))?;

        Ok(())
    }
}
