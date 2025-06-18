use std::time::Duration;

use schema::devlog::bitbridge::bit_bridge_cloud_service_client::BitBridgeCloudServiceClient;
use schema::devlog::bitbridge::commit_file_upload_request::UploadStatus;
use schema::devlog::bitbridge::{
    AddResourcesRequest,
    AddResourcesResponse,
    CancelSessionRequest,
    ClientUploadRequest,
    CloudResourceMessage,
    CommitFileUploadRequest,
    CreatePublicTransferSessionRequest,
    PublicTransferSessionMessage
};
use tonic::transport::Channel;
use tonic::Request;

use crate::config::get_gateway_grpc_url;
use crate::errors::NetworkError;
use crate::grpc::auth_provider::AuthProvider;
use crate::network::grpc_channel::GrpcChannel;

pub struct CloudServer {
    channel: GrpcChannel,
    auth_provider: AuthProvider
}

impl CloudServer {
    pub async fn new(auth_provider: AuthProvider) -> Self {
        Self {
            channel: GrpcChannel::new(Channel::builder(get_gateway_grpc_url().parse().unwrap()).timeout(Duration::from_millis(1200))),
            auth_provider
        }
    }

    pub async fn create_public_transfer_session(
        &self,
        password: Option<String>
    ) -> Result<PublicTransferSessionMessage, NetworkError> {
        let channel = self.channel.connect().await?;

        let request_body = CreatePublicTransferSessionRequest { password };

        let mut request = Request::new(request_body);

        self.auth_provider.with_auth(&mut request).await?;

        let mut cloud_rpc = BitBridgeCloudServiceClient::new(channel);
        let response = cloud_rpc.create_public_transfer_session(request).await?;

        Ok(response.into_inner().session)
    }

    pub async fn add_resources(
        &self,
        session_order_id: i64,
        resources: Vec<CloudResourceMessage>
    ) -> Result<AddResourcesResponse, NetworkError> {
        let channel = self.channel.connect().await?;

        let request_body = AddResourcesRequest {
            session_order_id,
            resources
        };

        let mut request = Request::new(request_body);
        self.auth_provider.with_auth(&mut request).await?;
        let mut cloud_rpc = BitBridgeCloudServiceClient::new(channel);
        let response = cloud_rpc.add_resources(request).await?;

        Ok(response.into_inner())
    }

    pub async fn commit_file_upload(
        &self,
        session_order_id: i64,
        resource_order_id: i64,
        status: UploadStatus,
        failed_reason: Option<String>
    ) -> Result<Option<ClientUploadRequest>, NetworkError> {
        let channel = self.channel.connect().await?;

        let request_body = CommitFileUploadRequest {
            session_order_id,
            resource_id: resource_order_id,
            status: status.into(),
            failed_reason
        };

        let mut request = Request::new(request_body);

        self.auth_provider.with_auth(&mut request).await?;

        let mut cloud_rpc = BitBridgeCloudServiceClient::new(channel);
        let response = cloud_rpc.commit_file_upload(request).await?;

        Ok(response.into_inner().next_upload_request)
    }

    pub async fn cancel_session(&self, session_order_id: i64) -> Result<(), NetworkError> {
        let channel = self.channel.connect().await?;

        let request_body = CancelSessionRequest { session_order_id };

        let mut request = Request::new(request_body);

        self.auth_provider.with_auth(&mut request).await?;

        let mut cloud_rpc = BitBridgeCloudServiceClient::new(channel);
        let _ = cloud_rpc.cancel_session(request).await?;

        Ok(())
    }
}
