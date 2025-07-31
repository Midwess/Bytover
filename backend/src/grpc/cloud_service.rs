use std::ops::Deref;
use std::pin::Pin;
use std::sync::Arc;
use surreal_devl::proxy::default::SurrealDeserializer;
use surrealdb::RecordId;
use surrealdb::rpc::format::json::req;
use surrealdb::sql::Value;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use schema::devlog::auth_gateway::models::User;
use schema::devlog::bitbridge::bit_bridge_cloud_service_server::BitBridgeCloudService;
use schema::devlog::bitbridge::cloud_resource_message::ResourceType as CloudResourceType;
use schema::devlog::bitbridge::commit_file_upload_request::UploadStatus;
use schema::devlog::bitbridge::{AddResourcesRequest, AddResourcesResponse, CancelSessionRequest, CancelSessionResponse, ClientUploadRequest, CommitFileUploadRequest, CommitFileUploadResponse, CreatePublicTransferSessionRequest, CreatePublicTransferSessionResponse, FindSessionRequest, FindSessionResponse, PublicSessionId, PublicTransferSessionMessage, SubscribeSessionInfoRequest, SubscribeSessionInfoResponse, UpdateTransferProgressRequest, UpdateTransferProgressResponse};
use schema::value::static_resource;
use tonic::{Request, Response, Status};
use tonic::codegen::tokio_stream;
use core_services::db::repository::abstraction::table::Table;
use core_services::db::surrealdb::id::SurrealDbId;
use devlog_sdk::live_query::live_query::{LiveId, LiveQuery};
use crate::cloud_storage::storage::CloudStorage;
use crate::entities::transfer_progress::TransferProgressStatus;
use crate::entities::transfer_resource::TransferResource;
use crate::entities::transfer_session::TransferSession;
use crate::repositories::transfer_session::{TransferSessionId, TransferSessionRepository};
use crate::transfer::transfer_service::{TransferResourceRequest, TransferService};

pub struct CloudGrpcService {
    pub transfer_service: TransferService,
    pub cloud_storage: Box<dyn CloudStorage>,
    pub session_repository: Box<dyn TransferSessionRepository>,
    pub live_query: Arc<LiveQuery>,
}

type SubscribeSessionResponseStream = Pin<Box<dyn tokio_stream::Stream<Item = Result<SubscribeSessionInfoResponse, Status>> + Send>>;

#[async_trait::async_trait]
impl BitBridgeCloudService for CloudGrpcService {
    type subscribe_session_infoStream = SubscribeSessionResponseStream;

    async fn find_session(&self, request: Request<FindSessionRequest>) -> Result<Response<FindSessionResponse>, Status> {
        let request = request.into_inner();
        let Some(alias) = request.alias else {
            return Err(Status::invalid_argument("Alias must be defined"))
        };

        let Some(session) = self.session_repository.find_session_by_alias(alias).await? else {
            return Ok(Response::new(FindSessionResponse::default()))
        };

        let response = FindSessionResponse {
            session: Some(PublicSessionId {
                order_id: session.order_id(),
                user_id: session.user_order_id()
            }),
            is_required_password: session.password().is_some()
        };

        Ok(Response::new(response))
    }

    async fn subscribe_session_info(&self, request: Request<SubscribeSessionInfoRequest>) -> Result<Response<Self::subscribe_session_infoStream>, Status> {
        let request = request.into_inner();
        let (tx, rx) = mpsc::channel(16);

        let session_id = TransferSessionId {
            order_id: Some(request.id.order_id ),
            user_order_id: Some(request.id.user_id ),
        };

        let tb = TransferSession::get_table();
        let thing = session_id.clone().id(tb);
        let live_id = LiveId::Record(thing);
        let live_stream = self.live_query.subscribe(live_id).await?;
        let mut subscription = live_stream.subscribe().await;
        let value = subscription.borrow().clone();

        let initial_session = match value {
            Some(value) => {
                Some(TransferSession::deserialize(&(value.data.into_inner())).map_err(|_| Status::internal("Cannot deserialize session"))?)
            }
            None => {
                self.session_repository.find_one(&session_id).await?
            }
        };

        let Some(initial_session) = initial_session else {
            return Err(Status::invalid_argument("Session not found or password is not correct"))
        };

        if !initial_session.validate_access(request.password) {
            return Err(Status::invalid_argument("Session not found or password is not correct"))
        }

        let is_completed = initial_session.is_completed();
        tx.send(Ok(SubscribeSessionInfoResponse {
            session: initial_session.into(),
        })).await.map_err(|_| Status::internal("Cannot send initial session"))?;

        if is_completed {
            log::info!("Session is completed");
            return Ok(Response::new(Box::pin(ReceiverStream::new(rx))))
        }

        tokio::spawn(async move {
            let mut stream = live_stream.subscribe().await;
            loop {
                if let Err(e) = stream.changed().await {
                    log::error!("Error: {}", e);
                    break;
                };

                let Some(value) = subscription.borrow().clone() else {
                   break;
                };

                let Ok(session) = TransferSession::deserialize(&value.data.into_inner()) else {
                    break;
                };

                let is_completed = session.is_completed();
                if tx.send(Ok(SubscribeSessionInfoResponse {
                    session: session.into(),
                })).await.is_err() {
                    log::error!("Cannot send session, closing");
                    break;
                }

                if is_completed {
                    log::info!("Session is completed");
                    break;
                }
            }
        });

        Ok(Response::new(Box::pin(ReceiverStream::new(rx)) as Self::subscribe_session_infoStream))
    }

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
            .create_public_transfer_session(user.order_id , request_body.password.clone())
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
                    order_id: Some(it.order_id ),
                    name: it.name.clone(),
                    r#type: (&schema_type).into(),
                    size: it.size as u64
                })
            })
            .collect::<Vec<_>>();

        let response = self
            .transfer_service
            .add_resources(user.order_id , request_body.session_order_id , requests)
            .await?;

        let mut source = response.first_resource.source();

        let signed_upload_url = self.cloud_storage.sign_upload(&mut source).await?;
        let response_body = AddResourcesResponse {
            first_resource_upload_request: ClientUploadRequest {
                resource_order_id: response.first_resource.order_id(),
                upload_url: signed_upload_url
            },
            thumbnail_upload_requests: response
                .thumbnails
                .iter()
                .filter_map(|(order_id, source)| match source.source.as_ref() {
                    Some(static_resource::static_resource::Source::Url(url)) => Some(ClientUploadRequest {
                        resource_order_id: *order_id,
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

        let resource_id = request_body.resource_id;
        let status = request_body.status();
        let err_msg = request_body.failed_reason.clone();

        let Some(next_resource) = self
            .transfer_service
            .commit_resource(
                user.order_id,
                request_body.session_order_id,
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
            .cancel_transfer(user.order_id , request_body.session_order_id )
            .await?;

        let response_body = CancelSessionResponse {};
        let response = Response::new(response_body);

        Ok(response)
    }

    async fn update_transfer_progress(&self, request: Request<UpdateTransferProgressRequest>) -> Result<Response<UpdateTransferProgressResponse>, Status> {
        let Some(user_id) = request.extensions().get::<User>().map(|it| it.order_id) else {
            return Err(Status::unauthenticated("Unauthenticated".to_owned()));
        };

        let request = request.into_inner();

        self.transfer_service.update_transfer_progress(
            user_id,
            request.session_order_id,
            request.resource_order_id,
            request.transferred_amount_in_bytes,
        ).await?;

        Ok(Response::new(UpdateTransferProgressResponse {}))
    }
}

impl CloudGrpcService {
    async fn create_upload_request(&self, resource: &TransferResource) -> Result<ClientUploadRequest, Status> {
        let mut source = resource.source();
        let url = self.cloud_storage.sign_upload(&mut source).await?;
        Ok(ClientUploadRequest {
            upload_url: url,
            resource_order_id: resource.order_id()
        })
    }
}
