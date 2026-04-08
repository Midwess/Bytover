use devlog_sdk::grpc_gateway::channel::GrpcGatewayChannel;
use schema::devlog::app_gateway::models::{Application, Device, User};
use schema::devlog::app_gateway::rpc::application_service_client::ApplicationServiceClient;
use schema::devlog::app_gateway::rpc::user_service_client::UserServiceClient;
use schema::devlog::app_gateway::rpc::{GenerateRandomAvatarRequest, MeRequest};
use tonic::metadata::MetadataValue;
use tonic::Request;

#[derive(Clone)]
pub struct AppGatewayClient {
    channel: GrpcGatewayChannel,
}

#[derive(Clone)]
#[allow(dead_code)]
pub struct AuthContext {
    pub user: User,
    pub device: Device,
    pub app: Application,
}

#[derive(Debug, thiserror::Error)]
pub enum AppGatewayError {
    #[error("Invalid authorization header")]
    InvalidAuthorization,
    #[error("{0}")]
    Unauthorized(String),
    #[error("{0}")]
    Upstream(String),
}

impl AppGatewayClient {
    pub fn new(channel: GrpcGatewayChannel) -> Self {
        Self { channel }
    }

    pub async fn resolve_auth(&self, authorization: &str) -> Result<AuthContext, AppGatewayError> {
        let channel = self
            .channel
            .connect()
            .await
            .map_err(|error| AppGatewayError::Upstream(error.to_string()))?;

        let mut client = UserServiceClient::new(channel);
        let mut request = Request::new(MeRequest { conditions: vec![] });
        let metadata = MetadataValue::try_from(authorization).map_err(|_| AppGatewayError::InvalidAuthorization)?;
        request.metadata_mut().insert("authorization", metadata);

        let response = client.me(request).await.map_err(|status| match status.code() {
            tonic::Code::Unauthenticated | tonic::Code::PermissionDenied => {
                AppGatewayError::Unauthorized(status.message().to_string())
            }
            _ => AppGatewayError::Upstream(status.to_string()),
        })?;

        let response = response.into_inner();

        Ok(AuthContext {
            user: response.user,
            device: response.device,
            app: response.app,
        })
    }

    pub async fn random_avatar(&self) -> Result<String, AppGatewayError> {
        let channel = self
            .channel
            .connect()
            .await
            .map_err(|error| AppGatewayError::Upstream(error.to_string()))?;

        let mut client = ApplicationServiceClient::new(channel);
        let response = client
            .get_avatar(GenerateRandomAvatarRequest {
                app_name: Some("BitBridge".to_string()),
            })
            .await
            .map_err(|status| AppGatewayError::Upstream(status.to_string()))?;

        Ok(response.into_inner().avatar.unwrap_or_default())
    }
}
