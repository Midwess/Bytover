use crate::entities::device::DeviceInfo;
use crate::entities::user::User;
use crate::protocol::rpc::auth_provider::AuthProvider;
use crate::protocol::rpc::connection::RpcNetworkModule;
use crate::protocol::rpc::errors::RpcErrors;
use core_services::utils::maybe::MaybeSend;
use schema::devlog::app_gateway::rpc::auth_service_client::AuthServiceClient;
use schema::devlog::app_gateway::rpc::authenticate_response::Action;
use schema::devlog::app_gateway::rpc::feedback_service_client::FeedbackServiceClient;
use schema::devlog::app_gateway::rpc::people_service_client::PeopleServiceClient;
use schema::devlog::app_gateway::rpc::user_service_client::UserServiceClient;
use schema::devlog::app_gateway::rpc::{AppFeedbackRequest, AuthenticateRequest, FindUserRequest, MeRequest};
use schema::devlog::bitbridge::p2p_orchestration_service_client::P2pOrchestrationServiceClient;
use schema::devlog::bitbridge::{
    CreateDeviceSessionRequest, FindP2pSessionRequest, GenAliasRequest, GenPeerRequest, GetDeviceAliasesRequest,
};
use schema::value::auth_method::AuthMethod;
use schema::value::device::RegisteringDevice;
use tonic::Request;

pub struct AppServer<T>
where
    T: Clone,
    T: tonic::client::GrpcService<tonic::body::Body>,
    T::Future: MaybeSend,
    T::Error: Into<tonic::codegen::StdError>,
    T::ResponseBody: http_body::Body<Data = bytes::Bytes> + Send + 'static,
    <T::ResponseBody as http_body::Body>::Error: Into<tonic::codegen::StdError> + Send,
{
    rpc_module: Box<dyn RpcNetworkModule<T>>,
    auth_provider: AuthProvider,
}

impl<T> AppServer<T>
where
    T: Clone,
    T: MaybeSend + Sync,
    T::Future: MaybeSend,
    T: tonic::client::GrpcService<tonic::body::Body>,
    T::Error: Into<tonic::codegen::StdError>,
    T::ResponseBody: http_body::Body<Data = bytes::Bytes> + Send + 'static,
    <T::ResponseBody as http_body::Body>::Error: Into<tonic::codegen::StdError> + Send,
{
    pub fn new(auth_provider: AuthProvider, network: Box<dyn RpcNetworkModule<T>>) -> Self {
        Self {
            auth_provider,
            rpc_module: network,
        }
    }

    pub async fn authenticate(&self, device: DeviceInfo) -> Result<String, RpcErrors> {
        let channel = self.rpc_module.connect().await?;
        let request = AuthenticateRequest {
            app_name: "BitBridge".to_string(),
            method: AuthMethod::Google.into(),
            device: RegisteringDevice {
                device_name: device.name,
                device_unique_key: device.unique_id,
                platform: device.platform.into(),
                device_type: device.device_type.into(),
                url: format!("{}/oauth", device.url),
            },
        };

        let auth_client = AuthServiceClient::new(channel);
        let response = auth_client.clone().authenticate(request).await.map(|it| it.into_inner())?;

        Ok(response
            .action
            .map(|it| match it {
                Action::OpenUrl(url) => url,
            })
            .clone()
            .unwrap_or_default())
    }

    pub async fn get_me(&self) -> Result<(User, String), RpcErrors> {
        let channel = self.rpc_module.connect().await?;
        let req = MeRequest { conditions: vec![] };
        let mut request = Request::new(req);

        self.auth_provider.with_auth(&mut request).await?;

        let user_client = UserServiceClient::new(channel);
        let response = user_client.clone().me(request).await.map(|it| it.into_inner())?;

        let user = User {
            id: response.user.order_id,
            email: response.user.email.clone(),
            name: response.user.display_name.clone(),
            avatar: response.user.avatar_url.clone().unwrap_or_default(),
        };
        let device_unique_key = response.device.unique_key.clone();

        Ok((user, device_unique_key))
    }

    pub async fn find_user(&self, user_order_id: u64) -> Result<Option<User>, RpcErrors> {
        let req = FindUserRequest {
            order_id: Some(user_order_id),
        };

        let request = Request::new(req);

        let channel = self.rpc_module.connect().await?;
        let mut client = PeopleServiceClient::new(channel);
        let response = client.find_user(request).await?;
        let Some(public_user) = response.into_inner().user else {
            return Ok(None);
        };

        let user = User {
            id: public_user.order_id.unwrap_or_default(),
            email: public_user.user_name.unwrap_or_default(),
            name: public_user.display_name.unwrap_or_default(),
            avatar: public_user.avatar_url.unwrap_or_default(),
        };

        Ok(Some(user))
    }

    pub async fn get_user_by_id(&self, user_id: u64) -> Result<User, RpcErrors> {
        self.find_user(user_id)
            .await?
            .ok_or_else(|| RpcErrors::BadRequest(format!("User {} not found", user_id)))
    }

    pub async fn feedback(&self, email: String, message: String) -> Result<(), RpcErrors> {
        let request = AppFeedbackRequest {
            app_name: "BitBridge".to_string(),
            user_email: Some(email),
            title: None,
            message,
        };

        let channel = self.rpc_module.connect().await?;
        let mut client = FeedbackServiceClient::new(channel);
        client.feedback_app(request).await?;
        Ok(())
    }

    pub async fn create_device_session(
        &self,
        alias: String,
        signalling_key: String,
        signalling_route: String,
    ) -> Result<schema::devlog::bitbridge::P2pSession, RpcErrors> {
        let channel = self.rpc_module.connect().await?;
        let req = CreateDeviceSessionRequest {
            alias,
            signalling_key,
            signalling_route,
        };
        let mut request = Request::new(req);

        self.auth_provider.with_auth(&mut request).await?;

        let mut client = P2pOrchestrationServiceClient::new(channel);
        let response = client.create_device_session(request).await?;
        Ok(response.into_inner().session)
    }

    pub async fn get_device_aliases(&self) -> Result<Vec<String>, RpcErrors> {
        let channel = self.rpc_module.connect().await?;
        let req = GetDeviceAliasesRequest {};
        let mut request = Request::new(req);

        self.auth_provider.with_auth(&mut request).await?;

        let mut client = P2pOrchestrationServiceClient::new(channel);
        let response = client.get_device_aliases(request).await?;
        Ok(response.into_inner().aliases)
    }

    pub async fn gen_alias(&self) -> Result<String, RpcErrors> {
        let channel = self.rpc_module.connect().await?;
        let req = GenAliasRequest {};
        let mut request = Request::new(req);

        self.auth_provider.with_auth(&mut request).await?;

        let mut client = P2pOrchestrationServiceClient::new(channel);
        let response = client.gen_alias(request).await?;
        Ok(response.into_inner().alias)
    }

    pub async fn find_p2p_session_by_alias(&self, alias: String) -> Result<Option<schema::devlog::bitbridge::P2pSession>, RpcErrors> {
        let channel = self.rpc_module.connect().await?;
        let req = FindP2pSessionRequest {
            key: Some(schema::devlog::bitbridge::find_p2p_session_request::Key::Alias(alias)),
        };
        let request = Request::new(req);

        let mut client = P2pOrchestrationServiceClient::new(channel);
        let response = client.find_session(request).await?;
        Ok(response.into_inner().session)
    }

    pub async fn gen_peer(&self, device: crate::entities::device::DeviceInfo) -> Result<crate::entities::peer::Peer, RpcErrors> {
        let channel = self.rpc_module.connect().await?;
        let req = GenPeerRequest {
            device: RegisteringDevice {
                platform: device.platform as i32,
                device_name: device.name.clone(),
                device_unique_key: device.unique_id.clone(),
                device_type: device.device_type as i32,
                url: device.url.clone(),
            },
        };
        let mut request = Request::new(req);

        let _ = self.auth_provider.with_auth(&mut request).await;

        let mut client = P2pOrchestrationServiceClient::new(channel);
        let response = client.gen_peer(request).await?.into_inner();

        let mut peer = crate::entities::peer::Peer::from(response.peer);
        peer.signalling_id = response.signalling_id;
        peer.region_code = response.region_code;
        peer.signalling_route = Some(response.signalling_route);

        Ok(peer)
    }
}
