use schema::devlog::auth_gateway::rpc::auth_service_client::AuthServiceClient;
use schema::devlog::auth_gateway::rpc::user_service_client::UserServiceClient;
use schema::devlog::auth_gateway::rpc::{MeRequest, SigninRequest};
use schema::value::auth_method::AuthMethod;
use schema::value::device::RegisteringDevice;
use std::time::Duration;
use tonic::transport::Channel;
use tonic::Request;

use crate::config::get_gateway_grpc_url;
use crate::entities::device::DeviceInfo;
use crate::entities::user::User;
use crate::errors::NetworkError;
use crate::grpc::auth_provider::AuthProvider;
use crate::network::grpc_channel::GrpcChannel;

pub struct AuthServer {
    channel: GrpcChannel,
    auth_provider: AuthProvider
}

impl AuthServer {
    pub async fn new(auth_provider: AuthProvider) -> Self {
        Self {
            channel: GrpcChannel::new(Channel::builder(get_gateway_grpc_url().parse().unwrap()).timeout(Duration::from_millis(1200))),
            auth_provider
        }
    }
}

impl AuthServer {
    pub async fn request_signin_url(&self, device: DeviceInfo) -> Result<String, NetworkError> {
        let channel = self.channel.connect().await?;

        let request = SigninRequest {
            app_name: "BitBridge".to_string(),
            method: AuthMethod::Google.into(),
            device: RegisteringDevice {
                device_name: device.name,
                device_unique_key: device.unique_id,
                platform: device.platform.into(),
                device_type: device.device_type.into()
            }
        };

        let mut auth_rpc = AuthServiceClient::new(channel);
        let response = auth_rpc.signin(request).await?;

        Ok(response.get_ref().signin_url.clone())
    }

    pub async fn get_me(&self) -> Result<User, NetworkError> {
        let channel = self.channel.connect().await?;

        let request = MeRequest { conditions: vec![] };

        // Create request and add bearer token
        let mut req = Request::new(request);
        self.auth_provider.with_auth(&mut req).await?;

        let mut user_rpc = UserServiceClient::new(channel);
        let response = user_rpc.me(req).await?;
        let response = response.get_ref();
        Ok(User {
            email: response.user.email.clone(),
            name: response.user.display_name.clone(),
            avatar: response.user.avatar_url.clone().unwrap_or_default()
        })
    }
}
