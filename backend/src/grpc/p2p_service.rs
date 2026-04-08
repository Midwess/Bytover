use crate::app_gateway::app_info::AppInfoService;
use crate::di_container::DiContainer;
use crate::repositories::p2p_session::P2PSessionRepository;
use crate::transfer::p2p_transfer_service::P2PTransferErrors;
use schema::devlog::app_gateway::models::{Device, User};
use schema::devlog::bitbridge::p2p_orchestration_service_server::P2pOrchestrationService;
use schema::devlog::bitbridge::{
    find_p2p_session_request,
    CreateDeviceSessionRequest,
    CreateDeviceSessionResponse,
    FindP2pSessionRequest,
    FindP2pSessionResponse,
    GetDeviceAliasesRequest,
    GetDeviceAliasesResponse,
    P2pSession
};
use std::sync::Arc;
use tonic::{Request, Response, Status};

pub struct P2PGrpcService {
    pub p2p_repository: Arc<dyn P2PSessionRepository>,
    pub app_service: Box<dyn AppInfoService>
}

fn normalize_signalling_route(signalling_route: &str) -> Result<String, Status> {
    let signalling_route = signalling_route.trim();

    if signalling_route.is_empty() {
        return Err(Status::invalid_argument("Signalling route must not be blank"));
    }

    Ok(signalling_route.to_string())
}

#[async_trait::async_trait]
impl P2pOrchestrationService for P2PGrpcService {
    async fn create_device_session(
        &self,
        request: Request<CreateDeviceSessionRequest>
    ) -> Result<Response<CreateDeviceSessionResponse>, Status> {
        let Some(user) = request.extensions().get::<User>() else {
            return Err(Status::unauthenticated("Unauthenticated".to_owned()));
        };

        let Some(device) = request.extensions().get::<Device>() else {
            return Err(Status::unauthenticated("Unauthenticated".to_owned()));
        };

        let request_body = request.get_ref();
        let alias = request_body.alias.clone();
        let signalling_key = request_body.signalling_key.clone();
        let signalling_route = normalize_signalling_route(&request_body.signalling_route)?;

        let p2p_transfer_service = DiContainer::instance().await.get_p2p_transfer_service().await;

        let session = p2p_transfer_service
            .create_user_device_session(
                user.order_id,
                device.order_id,
                device.name.clone(),
                alias,
                signalling_key,
                signalling_route
            )
            .await
            .map_err(|e| match e {
                P2PTransferErrors::AliasNotFound => Status::invalid_argument("Alias not found for this device"),
                _ => Status::internal(e.to_string())
            })?;

        let app = self.app_service.get_app_info("BitBridge".to_owned()).await?.unwrap();

        let response = CreateDeviceSessionResponse {
            session: P2pSession {
                session_id: session.session_id(),
                signalling_key: session.signalling_key().to_string(),
                owner_user_id: session.user_id(),
                description: session.description().map(|s| s.to_string()),
                access_url: session.access_url(app.web_url().to_string()),
                alias: session.alias().to_string(),
                signalling_route: session.signalling_route().to_string()
            }
        };

        Ok(Response::new(response))
    }

    async fn find_session(&self, request: Request<FindP2pSessionRequest>) -> Result<Response<FindP2pSessionResponse>, Status> {
        let request_body = request.into_inner();

        let alias = match request_body.key {
            Some(find_p2p_session_request::Key::Alias(alias)) => alias,
            None => return Err(Status::invalid_argument("Alias must be defined"))
        };

        let Some(session) = self.p2p_repository.find_by_alias(alias).await? else {
            return Ok(Response::new(FindP2pSessionResponse { session: None }));
        };

        let app = self.app_service.get_app_info("BitBridge".to_owned()).await?.unwrap();

        let response = FindP2pSessionResponse {
            session: Some(P2pSession {
                session_id: session.session_id(),
                signalling_key: session.signalling_key().to_string(),
                owner_user_id: session.user_id(),
                description: session.description().map(|s| s.to_string()),
                access_url: session.access_url(app.web_url().to_string()),
                alias: session.alias().to_string(),
                signalling_route: session.signalling_route().to_string()
            })
        };

        Ok(Response::new(response))
    }

    async fn get_device_aliases(
        &self,
        request: Request<GetDeviceAliasesRequest>
    ) -> Result<Response<GetDeviceAliasesResponse>, Status> {
        let Some(user) = request.extensions().get::<User>() else {
            return Err(Status::unauthenticated("Unauthenticated".to_owned()));
        };

        let Some(device) = request.extensions().get::<Device>() else {
            return Err(Status::unauthenticated("Unauthenticated".to_owned()));
        };

        let p2p_transfer_service = DiContainer::instance().await.get_p2p_transfer_service().await;

        let aliases = p2p_transfer_service
            .get_or_create_aliases(user.order_id, device.order_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(GetDeviceAliasesResponse { aliases }))
    }
}

#[cfg(test)]
mod tests {
    use super::normalize_signalling_route;
    use tonic::Code;

    #[test]
    fn trims_and_accepts_non_empty_signalling_route() {
        let signalling_route = normalize_signalling_route("  rpc-signalling-local  ").unwrap();

        assert_eq!(signalling_route, "rpc-signalling-local");
    }

    #[test]
    fn rejects_blank_signalling_route() {
        let error = normalize_signalling_route("   ").unwrap_err();

        assert_eq!(error.code(), Code::InvalidArgument);
        assert_eq!(error.message(), "Signalling route must not be blank");
    }
}
