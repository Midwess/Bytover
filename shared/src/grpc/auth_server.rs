use schema::{devlog::auth_gateway::rpc::{auth_service_client::AuthServiceClient, SigninRequest}, value::{auth_method::AuthMethod, device::RegisteringDevice}};
use tonic::transport::Channel;

use crate::{app::modules::environment::DeviceInfo, config::get_gateway_grpc_url, errors::AuthenticationError};

pub struct AuthServer {
    client: AuthServiceClient<Channel>
}

impl AuthServer {
    pub async fn new() -> Self {
        let client = AuthServiceClient::connect(get_gateway_grpc_url()).await.unwrap();

        Self { client }
    }
}

impl AuthServer {
    pub async fn request_signin_url(&self, device: DeviceInfo) -> Result<String, AuthenticationError> {
        let request = SigninRequest {
            app_name: "BitBridge".to_string(),
            method: AuthMethod::Google.into(),
            device: RegisteringDevice {
                device_name: device.name,
                device_unique_key: device.unique_id,
                platform: device.platform.into()
            }
        };

        let response = self.client.clone().signin(request).await?;

        Ok(response.get_ref().signin_url.clone())
    }
}