use crate::app_gateway::app_info::AppInfoService;
use crate::di_container::DiContainer;
use crate::repositories::p2p_session::P2PSessionRepository;
use schema::devlog::app_gateway::models::{Device, User};
use schema::devlog::bitbridge::p2p_orchestration_service_server::P2pOrchestrationService;
use schema::devlog::bitbridge::{
    CreateDeviceSessionRequest,
    CreateDeviceSessionResponse,
    FindP2pSessionRequest,
    FindP2pSessionResponse,
};
use std::sync::Arc;
use tonic::{Request, Response, Status};

pub struct P2PGrpcService {
    pub p2p_repository: Arc<dyn P2PSessionRepository>,
    pub app_service: Box<dyn AppInfoService>,
}

#[async_trait::async_trait]
impl P2pOrchestrationService for P2PGrpcService {
    async fn create_device_session(
        &self,
        request: Request<CreateDeviceSessionRequest>,
    ) -> Result<Response<CreateDeviceSessionResponse>, Status> {
        let Some(user) = request.extensions().get::<User>() else {
            return Err(Status::unauthenticated("Unauthenticated".to_owned()));
        };

        let Some(device) = request.extensions().get::<Device>() else {
            return Err(Status::unauthenticated("Unauthenticated".to_owned()));
        };

        let request_body = request.get_ref();

        let p2p_transfer_service = DiContainer::instance().await.get_p2p_transfer_service().await;

        let session = p2p_transfer_service
            .create_user_device_session(user.order_id, device.order_id, request_body.password_protected)
            .await?;

        let app = self.app_service.get_app_info("BitBridge".to_owned()).await?.unwrap();

        let response = CreateDeviceSessionResponse {
            session: schema::devlog::bitbridge::P2pSession {
                session_id: session.session_id(),
                signalling_room_id: session.owner_signalling_key(),
                owner_user_id: session.user_id(),
                password_protected: session.password_protected(),
                access_url: session.access_url(app.web_url().to_string()),
                alias: session.alias().to_string(),
                signalling_scope: session.get_scope().to_string(),
            },
        };

        Ok(Response::new(response))
    }

    async fn find_session(
        &self,
        request: Request<FindP2pSessionRequest>,
    ) -> Result<Response<FindP2pSessionResponse>, Status> {
        let request_body = request.into_inner();

        let alias = match request_body.key {
            Some(schema::devlog::bitbridge::find_p2p_session_request::Key::Alias(alias)) => alias,
            None => return Err(Status::invalid_argument("Alias must be defined")),
        };

        let Some(session) = self.p2p_repository.find_by_alias(alias).await? else {
            return Ok(Response::new(FindP2pSessionResponse { session: None }));
        };

        let app = self.app_service.get_app_info("BitBridge".to_owned()).await?.unwrap();

        let response = FindP2pSessionResponse {
            session: Some(schema::devlog::bitbridge::P2pSession {
                session_id: session.session_id(),
                signalling_room_id: session.member_signalling_key(),
                owner_user_id: session.user_id(),
                password_protected: session.password_protected(),
                access_url: session.access_url(app.web_url().to_string()),
                alias: session.alias().to_string(),
                signalling_scope: session.get_scope().to_string(),
            }),
        };

        Ok(Response::new(response))
    }
}
