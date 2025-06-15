use std::time::Duration;

use schema::devlog::bitbridge::bit_bridge_cloud_service_client::BitBridgeCloudServiceClient;
use schema::devlog::bitbridge::{
    CancelSessionRequest,
    CloudResourceMessage,
    CommitFileUploadRequest,
    CreatePublicTransferSessionRequest,
    CreatePublicTransferSessionResponse,
    UploadSession
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
        resources: Vec<CloudResourceMessage>,
        password: Option<String>
    ) -> Result<CreatePublicTransferSessionResponse, NetworkError> {
        let channel = self.channel.connect().await?;

        let request_body = CreatePublicTransferSessionRequest { password, resources };

        let mut request = Request::new(request_body);

        self.auth_provider.with_auth(&mut request).await?;

        let mut cloud_rpc = BitBridgeCloudServiceClient::new(channel);
        let response = cloud_rpc.create_public_transfer_session(request).await?;

        Ok(response.into_inner())
    }

    pub async fn commit_file_upload(
        &self,
        session_order_id: i64,
        upload_session: UploadSession
    ) -> Result<Option<UploadSession>, NetworkError> {
        let channel = self.channel.connect().await?;

        let request_body = CommitFileUploadRequest {
            session_order_id,
            upload_session
        };

        let mut request = Request::new(request_body);

        self.auth_provider.with_auth(&mut request).await?;

        let mut cloud_rpc = BitBridgeCloudServiceClient::new(channel);
        let response = cloud_rpc.commit_file_upload(request).await?;

        Ok(response.into_inner().next_upload)
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
