use crate::rpc::auth_provider::AuthProvider;
use crate::rpc::connection::RpcNetworkModule;
use crate::rpc::errors::RpcErrors;
use core_services::utils::maybe::MaybeSend;
use schema::devlog::bitbridge::bit_bridge_cloud_service_client::BitBridgeCloudServiceClient;
use schema::devlog::bitbridge::update_transfer_progress_request::Status;
use schema::devlog::bitbridge::{
    AddResourcesRequest,
    AddResourcesResponse,
    CancelSessionRequest,
    ClientUploadRequest,
    CloudResourceMessage,
    CreatePublicTransferSessionRequest,
    FindSessionRequest,
    FindSessionResponse,
    PublicSessionId,
    PublicTransferSessionMessage,
    SubscribeSessionInfoRequest,
    SubscribeSessionInfoResponse,
    UpdateTransferProgressRequest
};
use tonic::{Request, Streaming};

pub struct CloudServer<T>
where
    T: Clone,
    T: MaybeSend,
    T: tonic::client::GrpcService<tonic::body::Body>,
    T::Future: MaybeSend,
    T::Error: Into<tonic::codegen::StdError>,
    T::ResponseBody: http_body::Body<Data = bytes::Bytes> + 'static,
    <T::ResponseBody as http_body::Body>::Error: Into<tonic::codegen::StdError> + MaybeSend
{
    rpc_module: Box<dyn RpcNetworkModule<T>>,
    auth_provider: AuthProvider
}

impl<T> CloudServer<T>
where
    T: Clone,
    T: MaybeSend + Sync,
    T::Future: MaybeSend,
    T: tonic::client::GrpcService<tonic::body::Body>,
    T::Error: Into<tonic::codegen::StdError>,
    T::ResponseBody: http_body::Body<Data = bytes::Bytes> + Send + 'static,
    <T::ResponseBody as http_body::Body>::Error: Into<tonic::codegen::StdError> + Send
{
    pub fn new(rpc_module: Box<dyn RpcNetworkModule<T>>, auth_provider: AuthProvider) -> Self {
        Self { rpc_module, auth_provider }
    }

    pub async fn create_public_transfer_session(
        &self,
        password: Option<String>,
        to_emails: Vec<String>
    ) -> Result<PublicTransferSessionMessage, RpcErrors> {
        let request_body = CreatePublicTransferSessionRequest { password, to_emails };
        let channel = self.rpc_module.connect().await?;

        let mut request = Request::new(request_body);

        self.auth_provider.with_auth(&mut request).await?;

        let mut client = BitBridgeCloudServiceClient::new(channel);
        let response = client.create_public_transfer_session(request).await.map(|it| it.into_inner())?;

        Ok(response.session)
    }

    pub async fn add_resources(
        &self,
        session_order_id: u64,
        resources: Vec<CloudResourceMessage>
    ) -> Result<AddResourcesResponse, RpcErrors> {
        let channel = self.rpc_module.connect().await?;
        let request_body = AddResourcesRequest {
            session_order_id,
            resources
        };

        let mut request = Request::new(request_body);
        self.auth_provider.with_auth(&mut request).await?;
        let client = BitBridgeCloudServiceClient::new(channel);
        let response = client.clone().add_resources(request).await.map(|it| it.into_inner())?;

        Ok(response)
    }

    pub async fn cancel_session(&self, session_order_id: u64) -> Result<(), RpcErrors> {
        let channel = self.rpc_module.connect().await?;
        let request_body = CancelSessionRequest { session_order_id };

        let mut request = Request::new(request_body);

        self.auth_provider.with_auth(&mut request).await?;

        let client = BitBridgeCloudServiceClient::new(channel);
        let _ = client.clone().cancel_session(request).await.map(|it| it.into_inner())?;

        Ok(())
    }

    pub async fn update_transfer_progress(
        &self,
        session_order_id: u64,
        resource_order_id: u64,
        status: Status
    ) -> Result<Option<ClientUploadRequest>, RpcErrors> {
        let channel = self.rpc_module.connect().await?;
        let request_body = UpdateTransferProgressRequest {
            session_order_id,
            resource_id: resource_order_id,
            status: Some(status)
        };

        let mut request = Request::new(request_body);

        self.auth_provider.with_auth(&mut request).await?;

        let client = BitBridgeCloudServiceClient::new(channel);
        let next = client.clone().update_transfer_progress(request).await.map(|it| it.into_inner())?;

        Ok(next.next_upload_request)
    }

    pub async fn find_public_session(&self, alias: String) -> Result<FindSessionResponse, RpcErrors> {
        let channel = self.rpc_module.connect().await?;
        let request_body = FindSessionRequest { alias: Some(alias) };

        let mut client = BitBridgeCloudServiceClient::new(channel);
        let mut request = Request::new(request_body);
        self.auth_provider.with_auth(&mut request).await?;
        let response = client.find_session(request).await?;

        Ok(response.into_inner())
    }

    pub async fn subscribe_public_session_events(
        &self,
        user_id: u64,
        session_order_id: u64,
        password: Option<String>
    ) -> Result<Streaming<SubscribeSessionInfoResponse>, RpcErrors> {
        let channel = self.rpc_module.connect().await?;
        let mut client = BitBridgeCloudServiceClient::new(channel);
        let mut request = Request::new(SubscribeSessionInfoRequest {
            id: PublicSessionId {
                user_id,
                order_id: session_order_id
            },
            password
        });
        self.auth_provider.with_auth(&mut request).await?;
        let response = client.subscribe_session_info(request).await?;
        let response = response.into_inner();

        Ok(response)
    }
}
