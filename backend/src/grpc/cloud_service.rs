use crate::app_gateway::app_info::AppInfoService;
use crate::cloud_storage::storage::CloudStorage;
use crate::di_container::DiContainer;
use crate::entities::transfer_session::TransferSession;
use crate::repositories::transfer_session::{TransferSessionId, TransferSessionRepository};
use crate::transfer::transfer_service::TransferResourceRequest;
use crate::user::Token;
use devlog_sdk::live_query::live_query::LiveQuery;
use schema::devlog::auth_gateway::models::{Application, Device, User};
use schema::devlog::bitbridge::bit_bridge_cloud_service_server::BitBridgeCloudService;
use schema::devlog::bitbridge::cloud_resource_message::ResourceType as CloudResourceType;
use schema::devlog::bitbridge::subscribe_session_info_response::{Event, SessionUpdated};
use schema::devlog::bitbridge::{
    AddResourcesRequest,
    AddResourcesResponse,
    CancelSessionRequest,
    CancelSessionResponse,
    ClientUploadRequest,
    CompleteUploadPartRequest,
    CompleteUploadPartResponse,
    CreatePublicTransferSessionRequest,
    CreatePublicTransferSessionResponse,
    FindSessionRequest,
    FindSessionResponse,
    PublicSessionId,
    SubscribeSessionInfoRequest,
    SubscribeSessionInfoResponse,
    UpdateTransferProgressRequest,
    UpdateTransferProgressResponse
};
use std::pin::Pin;
use std::sync::Arc;
use sqlx::postgres::PgListener;
use sqlx::PgPool;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::codegen::tokio_stream;
use tonic::{Request, Response, Status};

pub struct CloudGrpcService {
    pub cloud_storage: Arc<dyn CloudStorage>,
    pub session_repository: Box<dyn TransferSessionRepository>,
    pub live_query: Arc<LiveQuery>,
    pub app_service: Box<dyn AppInfoService>,
    pub pg_pool: PgPool
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
            return Ok(Response::new(FindSessionResponse {
                session: None,
                access_url: "".to_string(),
                is_required_password: false
            }))
        };

        if session.is_failed() {
            return Ok(Response::new(FindSessionResponse {
                session: None,
                access_url: "".to_string(),
                is_required_password: false
            }))
        }

        let app = self.app_service.get_app_info("BitBridge".to_owned()).await?;
        let response = FindSessionResponse {
            session: Some(PublicSessionId {
                order_id: session.order_id(),
                user_id: session.user_order_id()
            }),
            access_url: session.access_url(app.unwrap().link),
            is_required_password: session.password().is_some()
        };

        Ok(Response::new(response))
    }

    async fn subscribe_session_info(
        &self,
        request: Request<SubscribeSessionInfoRequest>
    ) -> Result<Response<Self::subscribe_session_infoStream>, Status> {
        let request = request.into_inner();
        let (tx, rx) = mpsc::channel(16);

        let session_id = TransferSessionId {
            order_id: Some(request.id.order_id),
            user_order_id: Some(request.id.user_id)
        };

        let Some(initial_session) = self.session_repository.find_one(&session_id).await? else {
            return Err(Status::invalid_argument("Session not found or password is not correct"))
        };

        if !initial_session.validate_access(request.password.clone()) {
            return Err(Status::invalid_argument("Session not found or password is not correct"))
        }

        if initial_session.is_failed() {
            return Err(Status::invalid_argument("Session not found or password is not correct"))
        }

        let app = self.app_service.get_app_info("BitBridge".to_owned()).await?.unwrap();
        let is_completed = initial_session.is_completed();
        log::info!("Session: {:?}", initial_session);

        tx
            .send(Ok(SubscribeSessionInfoResponse {
                event: Some(Event::SessionUpdated(SessionUpdated {
                    session_updated: initial_session.into_msg(&self.cloud_storage, &app).await
                }))
            }))
            .await
            .map_err(|_| Status::internal("Cannot send initial session"))?;

        if is_completed {
            return Ok(Response::new(Box::pin(ReceiverStream::new(rx))))
        }

        let TransferSessionId { order_id, user_order_id } = session_id;
        let order_id = order_id.ok_or_else(|| Status::invalid_argument("Session id must be defined"))?;
        let user_order_id = user_order_id.ok_or_else(|| Status::invalid_argument("Session id must be defined"))?;

        let channel_name = format!("transfer_session_{}_{}", user_order_id, order_id);

        let mut listener = PgListener::connect_with(&self.pg_pool).await.map_err(|err| {
            log::error!("Failed to connect PgListener: {err}");
            Status::internal("Unable to connect to session notifications")
        })?;

        if let Err(err) = listener.listen(&channel_name).await {
            log::error!("Failed to listen on channel {channel_name}: {err}");
            return Err(Status::internal("Unable to subscribe to session updates"));
        }

        let cloud_storage = self.cloud_storage.clone();
        let app = app.clone();
        let tx_updates = tx.clone();
        let mut current_session = initial_session;

        tokio::spawn(async move {
            let mut listener = listener;

            loop {
                let notification = match listener.recv().await {
                    Ok(notification) => notification,
                    Err(err) => {
                        log::error!("Failed to receive notification: {err}");
                        break;
                    }
                };

                let payload = notification.payload();
                let session: TransferSession = match serde_json::from_str(payload) {
                    Ok(session) => session,
                    Err(err) => {
                        log::error!("Failed to deserialize session payload: {err}");
                        continue;
                    }
                };

                let events = current_session.get_change_events(&session, &cloud_storage, &app).await;
                for event in events {
                    if tx_updates
                        .send(Ok(SubscribeSessionInfoResponse { event: Some(event) }))
                        .await
                        .is_err()
                    {
                        log::info!("Client disconnected from session info stream");
                        return;
                    }
                }

                current_session = session;

                if current_session.is_completed() {
                    log::info!("Session is completed, closing stream...");
                    break;
                }
            }
        });

        Ok(Response::new(
            Box::pin(ReceiverStream::new(rx)) as Self::subscribe_session_infoStream
        ))
    }

    async fn create_public_transfer_session(
        &self,
        request: Request<CreatePublicTransferSessionRequest>
    ) -> Result<Response<CreatePublicTransferSessionResponse>, Status> {
        let Some(token) = request.extensions().get::<Token>() else {
            return Err(Status::unauthenticated("Unauthenticated".to_owned()));
        };

        let Some(user) = request.extensions().get::<User>() else {
            return Err(Status::unauthenticated("Unauthenticated".to_owned()));
        };

        let Some(app) = request.extensions().get::<Application>() else {
            return Err(Status::unauthenticated("Unauthenticated".to_owned()));
        };

        let transfer_service = DiContainer::instance().await.get_transfer_service(token.clone()).await;
        let request_body = request.get_ref();
        let new_session = transfer_service
            .create_public_transfer_session(user, request_body.password.clone(), request_body.to_emails.clone())
            .await?;

        let response_body = CreatePublicTransferSessionResponse {
            session: new_session.into_msg(&self.cloud_storage, app).await
        };

        let response = Response::new(response_body);
        Ok(response)
    }

    async fn add_resources(&self, request: Request<AddResourcesRequest>) -> Result<Response<AddResourcesResponse>, Status> {
        let Some(token) = request.extensions().get::<Token>() else {
            return Err(Status::unauthenticated("Unauthenticated".to_owned()));
        };

        let Some(device) = request.extensions().get::<Device>() else {
            return Err(Status::unauthenticated("Unauthenticated".to_owned()));
        };

        let Some(user) = request.extensions().get::<User>() else {
            return Err(Status::unauthenticated("Unauthenticated".to_owned()));
        };

        let Some(app) = request.extensions().get::<Application>() else {
            return Err(Status::unauthenticated("Unauthenticated".to_owned()));
        };

        let transfer_service = DiContainer::instance().await.get_transfer_service(token.clone()).await;
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
                    order_id: Some(it.order_id),
                    name: it.name.clone(),
                    r#type: (&schema_type).into(),
                    size: it.size as u64
                })
            })
            .collect::<Vec<_>>();

        let response = transfer_service.add_resources(user, device, app, request_body.session_order_id, requests).await?;

        let response_body = AddResourcesResponse {
            first_resource_upload_request: ClientUploadRequest {
                resource_order_id: response.first_resource.order_id(),
                upload: Some(response.first_resource_upload_request)
            },
            thumbnail_upload_requests: response
                .thumbnail_upload_urls
                .iter()
                .map(|(order_id, url)| ClientUploadRequest {
                    resource_order_id: *order_id,
                    upload: Some(schema::devlog::bitbridge::client_upload_request::Upload::SingleUrl(url.clone()))
                })
                .collect::<Vec<_>>()
        };

        let response = Response::new(response_body);

        Ok(response)
    }

    async fn cancel_session(&self, request: Request<CancelSessionRequest>) -> Result<Response<CancelSessionResponse>, Status> {
        let Some(token) = request.extensions().get::<Token>() else {
            return Err(Status::unauthenticated("Unauthenticated".to_owned()));
        };

        let Some(user) = request.extensions().get::<User>() else {
            return Err(Status::unauthenticated("Unauthenticated".to_owned()));
        };

        let transfer_service = DiContainer::instance().await.get_transfer_service(token.clone()).await;
        let request_body = request.get_ref();

        transfer_service.cancel_transfer(user.order_id, request_body.session_order_id).await?;

        let response_body = CancelSessionResponse {};
        let response = Response::new(response_body);

        Ok(response)
    }

    async fn update_transfer_progress(
        &self,
        request: Request<UpdateTransferProgressRequest>
    ) -> Result<Response<UpdateTransferProgressResponse>, Status> {
        let Some(token) = request.extensions().get::<Token>() else {
            return Err(Status::unauthenticated("Unauthenticated".to_owned()));
        };

        let Some(user) = request.extensions().get::<User>() else {
            return Err(Status::unauthenticated("Unauthenticated".to_owned()));
        };

        let Some(device) = request.extensions().get::<Device>() else {
            return Err(Status::unauthenticated("Unauthenticated".to_owned()));
        };

        let transfer_service = DiContainer::instance().await.get_transfer_service(token.clone()).await;
        let request = request.get_ref();
        let Some(status) = request.status.as_ref() else {
            return Err(Status::invalid_argument("Status must be defined"));
        };

        let Some((next_resource_id, next_upload_request)) = transfer_service
            .update_transfer_progress(user, device, request.session_order_id, request.resource_id, status)
            .await?
        else {
            return Ok(Response::new(UpdateTransferProgressResponse { next_upload_request: None }))
        };

        Ok(Response::new(UpdateTransferProgressResponse {
            next_upload_request: Some(ClientUploadRequest {
                resource_order_id: next_resource_id,
                upload: Some(next_upload_request.clone())
            })
        }))
    }

    async fn complete_upload_part(
        &self,
        request: Request<CompleteUploadPartRequest>
    ) -> Result<Response<CompleteUploadPartResponse>, Status> {
        let Some(user) = request.extensions().get::<User>() else {
            return Err(Status::unauthenticated("Unauthenticated".to_owned()));
        };

        let context = &request.get_ref().context_token;

        let upload = self.cloud_storage.complete_upload_part(user, context).await?;

        let response = Response::new(CompleteUploadPartResponse { part: upload });

        Ok(response)
    }
}
