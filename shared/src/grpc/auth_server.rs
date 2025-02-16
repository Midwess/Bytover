use schema::{devlog::auth_gateway::rpc::{auth_service_client::AuthServiceClient, SigninRequest}, value::{auth_method::AuthMethod, device::RegisteringDevice}};
use tonic::transport::Channel;

use crate::{app::{modules::environment::DeviceInfo, ports::authentication_service::{AuthenticationServer, AuthenticationServerError}}, config::get_gateway_grpc_url, TOKIO_RT};

pub struct AuthServer {
    client: AuthServiceClient<Channel>
}

impl AuthServer {
    pub async fn new() -> Self {
        let client = AuthServiceClient::connect(get_gateway_grpc_url()).await.unwrap();

        Self { client }
    }
}

#[async_trait::async_trait]
impl AuthenticationServer for AuthServer {
    async fn request_signin_url(&self, device: DeviceInfo) -> Result<String, AuthenticationServerError> {
        let request = SigninRequest {
            app_name: "BitBridge".to_string(),
            method: AuthMethod::Google.into(),
            device: RegisteringDevice {
                device_name: device.name,
                device_unique_key: device.unique_id,
                platform: device.platform.into()
            }
        };

        log::info!(target: "auth_server", "Requesting authorization url");
        let response = self.client.clone().signin(request).await?;
        log::info!(target: "auth_server", "Received authorization url");

        Ok(response.get_ref().signin_url.clone())
    }
}