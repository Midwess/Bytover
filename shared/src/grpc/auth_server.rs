use core_services::db::repository::abstraction::local_repository::LocalSurrealDbRepository;
use schema::{devlog::auth_gateway::rpc::{auth_service_client::AuthServiceClient, user_service_client::UserServiceClient, MeRequest, SigninRequest}, value::{auth_method::AuthMethod, device::RegisteringDevice}};
use tokio::sync::Mutex;
use tonic::{client::GrpcService, transport::{channel, Channel, Endpoint}};
use std::{str::FromStr, sync::{Arc}, time::Duration};
use tonic::metadata::{MetadataValue, MetadataMap};
use tonic::Request;

use crate::{app::modules::environment::DeviceInfo, config::get_gateway_grpc_url, entities::{session::SessionType, user::User}, errors::NetworkError, network::{grpc_channel::GrpcChannel, module::{InternetConnection, NetworkModule}}, persistence::session::{SessionId, SessionRepository}};

pub struct AuthServer {
    channel: GrpcChannel,
    session_repository: SessionRepository,
}

impl AuthServer {
    pub async fn new(session_repository: SessionRepository) -> Self {
        Self { 
            channel: GrpcChannel::new(Channel::builder(get_gateway_grpc_url().parse().unwrap()).timeout(Duration::from_millis(1200))),
            session_repository,
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
                platform: device.platform.into()
            }
        };

        let mut auth_rpc = AuthServiceClient::new(channel);
        let response = auth_rpc.signin(request).await?;

        Ok(response.get_ref().signin_url.clone())
    }

    pub async fn get_me(&self) -> Result<User, NetworkError> {
        let channel = self.channel.connect().await?;

        let request =  MeRequest {};

        // Create request and add bearer token
        let mut req = Request::new(request);
        self.with_auth(&mut req).await?;

        let mut user_rpc = UserServiceClient::new(channel);
        let response = user_rpc.me(req).await?;
        let response = response.get_ref();
        Ok(User { 
            email: response.user.email.clone(), 
            name: response.user.display_name.clone(), 
            avatar: response.user.avatar_url.clone().unwrap_or_default() 
        })
    }

    // Helper method to create authenticated request
    async fn with_auth<T>(&self, request: &mut Request<T>) -> Result<(), NetworkError> {
        let session = self.session_repository.find_one(&SessionId {r#type: SessionType::Access})
            .await.map_err(|e| NetworkError::Unauthorized(e.to_string()))?;

        log::info!("Session: {:?}", session);

        if session.is_none() {
            return Err(NetworkError::Unauthorized("Session not found".to_string()));
        }

        let token = session.unwrap().token;

        if let Ok(token) = MetadataValue::from_str(&token.value) {
            request.metadata_mut().insert("authorization", token);
        }

        Ok(())
    }
}
