use schema::devlog::bitbridge::bit_bridge_cloud_service_server::BitBridgeCloudService;
use schema::devlog::bitbridge::{
    CancelSessionRequest,
    CancelSessionResponse,
    CommitFileUploadRequest,
    CommitFileUploadResponse,
    CreatePublicTransferSessionRequest,
    CreatePublicTransferSessionResponse
};
use tonic::{Request, Response, Status};

pub struct CloudGrpcService {}

#[async_trait::async_trait]
impl BitBridgeCloudService for CloudGrpcService {
    async fn create_public_transfer_session(
        &self,
        request: Request<CreatePublicTransferSessionRequest>
    ) -> Result<Response<CreatePublicTransferSessionResponse>, Status> {
        todo!()
    }

    async fn commit_file_upload(
        &self,
        request: Request<CommitFileUploadRequest>
    ) -> Result<Response<CommitFileUploadResponse>, Status> {
        todo!()
    }

    async fn cancel_session(&self, request: Request<CancelSessionRequest>) -> Result<Response<CancelSessionResponse>, Status> {
        todo!()
    }
}
