use crate::entities::device::DeviceInfo;
use crate::entities::user::User;
use crate::rpc::auth_provider::AuthProvider;
use crate::rpc::connection::RpcNetworkModule;
use crate::rpc::errors::RpcErrors;
use core_services::utils::maybe::MaybeSend;
use schema::devlog::auth_gateway::rpc::auth_service_client::AuthServiceClient;
use schema::devlog::auth_gateway::rpc::user_service_client::UserServiceClient;
use schema::devlog::auth_gateway::rpc::{FindUserRequest, MeRequest, SigninRequest};
use schema::value::auth_method::AuthMethod;
use schema::value::device::RegisteringDevice;
use tonic::Request;
use schema::devlog::auth_gateway::rpc::people_service_client::PeopleServiceClient;

pub struct AuthServer<T>
where
    T: Clone,
    T: tonic::client::GrpcService<tonic::body::Body>,
    T::Future: MaybeSend,
    T::Error: Into<tonic::codegen::StdError>,
    T::ResponseBody: http_body::Body<Data = bytes::Bytes> + Send + 'static,
    <T::ResponseBody as http_body::Body>::Error: Into<tonic::codegen::StdError> + Send
{
    rpc_module: Box<dyn RpcNetworkModule<T>>,
    auth_provider: AuthProvider
}

impl<T> AuthServer<T>
where
    T: Clone,
    T: MaybeSend + Sync,
    T::Future: MaybeSend,
    T: tonic::client::GrpcService<tonic::body::Body>,
    T::Error: Into<tonic::codegen::StdError>,
    T::ResponseBody: http_body::Body<Data = bytes::Bytes> + Send + 'static,
    <T::ResponseBody as http_body::Body>::Error: Into<tonic::codegen::StdError> + Send
{
    pub fn new(auth_provider: AuthProvider, network: Box<dyn RpcNetworkModule<T>>) -> Self {
        Self {
            auth_provider,
            rpc_module: network
        }
    }

    pub async fn request_signin_url(&self, device: DeviceInfo) -> Result<String, RpcErrors> {
        let channel = self.rpc_module.connect().await?;
        let request = SigninRequest {
            app_name: "BitBridge".to_string(),
            method: AuthMethod::Google.into(),
            device: RegisteringDevice {
                device_name: device.name,
                device_unique_key: device.unique_id,
                platform: device.platform.into(),
                device_type: device.device_type.into()
            },
            original_web_page_url: None
        };

        let auth_client = AuthServiceClient::new(channel);
        let response = auth_client.clone().signin(request).await.map(|it| it.into_inner())?;

        Ok(response.signin_url.clone())
    }

    pub async fn get_me(&self) -> Result<User, RpcErrors> {
        let channel = self.rpc_module.connect().await?;
        let req = MeRequest { conditions: vec![] };
        let mut request = Request::new(req);

        // Create request and add bearer token
        self.auth_provider.with_auth(&mut request).await?;

        let user_client = UserServiceClient::new(channel);
        let response = user_client.clone().me(request).await.map(|it| it.into_inner())?;

        Ok(User {
            id: response.user.order_id,
            email: response.user.email.clone(),
            name: response.user.display_name.clone(),
            avatar: response.user.avatar_url.clone().unwrap_or_default()
        })
    }

    pub async fn find_user(&self, user_order_id: u64) -> Result<Option<User>, RpcErrors> {
        let req = FindUserRequest {
            order_id: Some(user_order_id),
        };

        let mut request = Request::new(req);
        self.auth_provider.with_auth(&mut request).await?;

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
            avatar: public_user.avatar_url.unwrap_or_default()
        };

        Ok(Some(user))
    }
}
