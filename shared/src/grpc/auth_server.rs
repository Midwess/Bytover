use crate::config::get_gateway_grpc_url;
use crate::entities::device::DeviceInfo;
use crate::entities::user::User;
use crate::errors::NetworkError;
use crate::grpc::auth_provider::AuthProvider;
use crate::network::grpc_channel::GrpcClient;
use schema::devlog::auth_gateway::rpc::auth_service_client::AuthServiceClient;
use schema::devlog::auth_gateway::rpc::user_service_client::UserServiceClient;
use schema::devlog::auth_gateway::rpc::{MeRequest, SigninRequest};
use schema::value::auth_method::AuthMethod;
use schema::value::device::RegisteringDevice;
use tokio::task::spawn_local;
use tonic::Request;

pub struct AuthServer {
    client: GrpcClient,
    auth_provider: AuthProvider
}

impl AuthServer {
    pub async fn new(auth_provider: AuthProvider) -> Self {
        Self {
            client: GrpcClient::new(get_gateway_grpc_url()),
            auth_provider
        }
    }
}

impl AuthServer {
    pub async fn request_signin_url(&self, device: DeviceInfo) -> Result<String, NetworkError> {
        let client = self.client.connect().await?;

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

        let mut auth_rpc = AuthServiceClient::new(client);
        let response = spawn_local(async move { auth_rpc.signin(request).await.map(|it| it.into_inner()) })
            .await
            .unwrap()?;

        Ok(response.signin_url.clone())
    }

    pub async fn get_me(&self) -> Result<User, NetworkError> {
        let client = self.client.connect().await?;

        let req = MeRequest { conditions: vec![] };
        let mut request = Request::new(req);

        // Create request and add bearer token
        self.auth_provider.with_auth(&mut request).await?;

        let mut user_rpc = UserServiceClient::new(client);
        let response = spawn_local(async move { user_rpc.me(request).await.map(|it| it.into_inner()) }).await.unwrap()?;

        Ok(User {
            email: response.user.email.clone(),
            name: response.user.display_name.clone(),
            avatar: response.user.avatar_url.clone().unwrap_or_default()
        })
    }
}
