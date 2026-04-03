use devlog_sdk::grpc_gateway::channel::GrpcGatewayChannel;
use schema::devlog::app_gateway::models::{Application, Device, User};
use schema::devlog::app_gateway::rpc::user_service_client::UserServiceClient;
use tonic::metadata::MetadataValue;
use tonic::Request;

#[derive(Clone)]
#[allow(dead_code)]
pub struct AuthContext {
    pub user: User,
    pub app: Application,
    pub device: Device,
    pub token: String,
}

#[allow(dead_code)]
pub enum AuthError {
    MissingToken,
    InvalidToken(String),
    GrpcError(String),
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::MissingToken => write!(f, "Missing token"),
            AuthError::InvalidToken(e) => write!(f, "Invalid token: {}", e),
            AuthError::GrpcError(e) => write!(f, "gRPC error: {}", e),
        }
    }
}

#[derive(Clone)]
pub struct AppGatewayClient {
    channel: GrpcGatewayChannel,
}

impl AppGatewayClient {
    pub fn new(channel: GrpcGatewayChannel) -> Self {
        Self {
            channel,
        }
    }

    pub async fn validate_token(&self, token: &str) -> Result<AuthContext, AuthError> {
        let channel = self
            .channel
            .connect()
            .await
            .map_err(|e| AuthError::GrpcError(e.to_string()))?;

        let mut client = UserServiceClient::new(channel);

        let mut request = Request::new(schema::devlog::app_gateway::rpc::MeRequest {
            conditions: vec![],
        });

        let token_str = format!("Bearer {}", token);
        let metadata = request.metadata_mut();
        metadata.insert(
            "authorization",
            MetadataValue::try_from(token_str).map_err(|e| AuthError::InvalidToken(e.to_string()))?,
        );

        let response = client
            .me(request)
            .await
            .map_err(|e| AuthError::GrpcError(e.to_string()))?;

        let response = response.into_inner();

        Ok(AuthContext {
            user: response.user,
            app: response.app,
            device: response.device,
            token: token.to_string(),
        })
    }
}

