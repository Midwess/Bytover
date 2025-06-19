use schema::devlog::auth_gateway::models::User;
use schema::devlog::bitbridge::bit_bridge_cloud_service_server::BitBridgeCloudService;
use schema::devlog::bitbridge::cloud_resource_message::ResourceType as CloudResourceType;
use schema::devlog::bitbridge::commit_file_upload_request::UploadStatus;
use schema::devlog::bitbridge::{
    AddResourcesRequest,
    AddResourcesResponse,
    CancelSessionRequest,
    CancelSessionResponse,
    ClientUploadRequest,
    CommitFileUploadRequest,
    CommitFileUploadResponse,
    CreatePublicTransferSessionRequest,
    CreatePublicTransferSessionResponse
};
use schema::value::static_resource;
use tonic::{Request, Response, Status};

use crate::cloud_storage::storage::CloudStorage;
use crate::entities::transfer_progress::TransferProgressStatus;
use crate::entities::transfer_resource::TransferResource;
use crate::transfer::transfer_service::{TransferResourceRequest, TransferService};

pub struct CloudGrpcService {
    pub transfer_service: TransferService,
    pub cloud_storage: Box<dyn CloudStorage>
}

#[async_trait::async_trait]
impl BitBridgeCloudService for CloudGrpcService {
    async fn create_public_transfer_session(
        &self,
        request: Request<CreatePublicTransferSessionRequest>
    ) -> Result<Response<CreatePublicTransferSessionResponse>, Status> {
        let Some(user) = request.extensions().get::<User>() else {
            return Err(Status::unauthenticated("Unauthenticated".to_owned()));
        };

        let request_body = request.get_ref();
        let new_session = self
            .transfer_service
            .create_public_transfer_session(user.order_id as u64, request_body.password.clone())
            .await?;

        let response_body = CreatePublicTransferSessionResponse {
            session: new_session.into()
        };

        let response = Response::new(response_body);
        return Ok(response)
    }

    async fn add_resources(&self, request: Request<AddResourcesRequest>) -> Result<Response<AddResourcesResponse>, Status> {
        let Some(user) = request.extensions().get::<User>() else {
            return Err(Status::unauthenticated("Unauthenticated".to_owned()));
        };

        let request_body = request.get_ref();
        let requests = request_body
            .resources
            .iter()
            .filter_map(|it| {
                let Ok(schema_type) = CloudResourceType::try_from(it.r#type) else {
                    log::warn!("The type {} is not supported", it.r#type);
                    return None
                };

                Some(TransferResourceRequest {
                    order_id: Some(it.order_id as u64),
                    name: it.name.clone(),
                    r#type: (&schema_type).into(),
                    size: it.size as u64
                })
            })
            .collect::<Vec<_>>();

        let response = self
            .transfer_service
            .add_resources(user.order_id as u64, request_body.session_order_id as u64, requests)
            .await?;

        let mut source = response.first_resource.source();

        let signed_upload_url = self.cloud_storage.sign_upload(&mut source).await?;
        let response_body = AddResourcesResponse {
            first_resource_upload_request: ClientUploadRequest {
                resource_order_id: response.first_resource.order_id() as i64,
                upload_url: signed_upload_url
            },
            thumbnail_upload_requests: response
                .thumbnails
                .iter()
                .filter_map(|(order_id, source)| match source.source.as_ref() {
                    Some(static_resource::static_resource::Source::Url(url)) => Some(ClientUploadRequest {
                        resource_order_id: *order_id as i64,
                        upload_url: url.clone()
                    }),
                    _ => None
                })
                .collect::<Vec<_>>()
        };

        let response = Response::new(response_body);

        Ok(response)
    }

    async fn commit_file_upload(
        &self,
        request: Request<CommitFileUploadRequest>
    ) -> Result<Response<CommitFileUploadResponse>, Status> {
        let Some(user) = request.extensions().get::<User>() else {
            return Err(Status::unauthenticated("Unauthenticated".to_owned()));
        };

        let request_body = request.get_ref();

        let resource_id = request_body.resource_id as u64;
        let status = request_body.status();
        let err_msg = request_body.failed_reason.clone();

        let Some(next_resource) = self
            .transfer_service
            .commit_resource(
                user.order_id as u64,
                request_body.session_order_id as u64,
                resource_id,
                match status {
                    UploadStatus::Failed => TransferProgressStatus::Failed(err_msg.unwrap_or("Unknown".to_owned())),
                    UploadStatus::Success => TransferProgressStatus::Success
                }
            )
            .await?
        else {
            let response_body = CommitFileUploadResponse { next_upload_request: None };

            let response = Response::new(response_body);
            return Ok(response)
        };

        let response_body = CommitFileUploadResponse {
            next_upload_request: Some(self.create_upload_request(&next_resource).await?)
        };

        let response = Response::new(response_body);

        Ok(response)
    }

    async fn cancel_session(&self, request: Request<CancelSessionRequest>) -> Result<Response<CancelSessionResponse>, Status> {
        let Some(user) = request.extensions().get::<User>() else {
            return Err(Status::unauthenticated("Unauthenticated".to_owned()));
        };

        let request_body = request.get_ref();

        self.transfer_service
            .cancel_transfer(user.order_id as u64, request_body.session_order_id as u64)
            .await?;

        let response_body = CancelSessionResponse {};
        let response = Response::new(response_body);

        Ok(response)
    }
}

impl CloudGrpcService {
    async fn create_upload_request(&self, resource: &TransferResource) -> Result<ClientUploadRequest, Status> {
        let mut source = resource.source();
        let url = self.cloud_storage.sign_upload(&mut source).await?;
        Ok(ClientUploadRequest {
            upload_url: url,
            resource_order_id: resource.order_id() as i64
        })
    }
}
