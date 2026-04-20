use crate::app_gateway::app_info::AppInfoService;
use crate::di_container::DiContainer;
use crate::repositories::p2p_session::P2PSessionRepository;
use crate::transfer::p2p_transfer_service::P2PTransferErrors;
use schema::devlog::app_gateway::models::{Device, User};
use schema::devlog::bitbridge::p2p_orchestration_service_server::P2pOrchestrationService;
use schema::devlog::bitbridge::{
    find_p2p_session_request, CreateDeviceSessionRequest, CreateDeviceSessionResponse, FindP2pSessionRequest, FindP2pSessionResponse,
    GenPeerRequest, GenPeerResponse, GetDeviceAliasesRequest, GetDeviceAliasesResponse, GetRegionRequest, GetRegionResponse,
    P2pSession, PeerMessage,
};
use std::sync::Arc;
use tonic::{Request, Response, Status};

pub struct P2PGrpcService {
    pub p2p_repository: Arc<dyn P2PSessionRepository>,
    pub app_service: Box<dyn AppInfoService>,
}

const DEFAULT_REGION_CODE: &str = "local";

fn normalize_signalling_route(signalling_route: &str) -> Result<String, Status> {
    let signalling_route = signalling_route.trim();

    if signalling_route.is_empty() {
        return Err(Status::invalid_argument("Signalling route must not be blank"));
    }

    Ok(signalling_route.to_string())
}

fn derive_signalling_region(region_code: Option<&str>) -> (String, String) {
    let region_code = region_code
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(DEFAULT_REGION_CODE)
        .to_string();

    let signalling_route = format!("rpc-signalling-{region_code}");

    (region_code, signalling_route)
}

fn current_region_code() -> Option<String> {
    resolve_region_code(
        std::env::var("BYTOVER_REGION_CODE").ok().as_deref(),
        normalize_railway_region(std::env::var("RAILWAY_REPLICA_REGION").ok().as_deref()).as_deref(),
    )
}

fn resolve_region_code(bytover_region_code: Option<&str>, railway_replica_region: Option<&str>) -> Option<String> {
    [
        bytover_region_code,
        railway_replica_region,
    ]
    .into_iter()
    .flatten()
    .map(str::trim)
    .find(|value| !value.is_empty())
    .map(str::to_string)
}

fn normalize_railway_region(region: Option<&str>) -> Option<String> {
    region
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.split('-').find(|segment| !segment.is_empty()).unwrap_or(value).to_string())
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
                signalling_route,
            )
            .await
            .map_err(|e| match e {
                P2PTransferErrors::AliasNotFound => Status::invalid_argument("Alias not found for this device"),
                _ => Status::internal(e.to_string()),
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
                signalling_route: session.signalling_route().to_string(),
            },
        };

        Ok(Response::new(response))
    }

    async fn find_session(&self, request: Request<FindP2pSessionRequest>) -> Result<Response<FindP2pSessionResponse>, Status> {
        let request_body = request.into_inner();

        let alias = match request_body.key {
            Some(find_p2p_session_request::Key::Alias(alias)) => alias,
            None => return Err(Status::invalid_argument("Alias must be defined")),
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
                signalling_route: session.signalling_route().to_string(),
            }),
        };

        Ok(Response::new(response))
    }

    async fn get_device_aliases(
        &self,
        request: Request<GetDeviceAliasesRequest>,
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

    async fn gen_peer(&self, request: Request<GenPeerRequest>) -> Result<Response<GenPeerResponse>, Status> {
        let user = request.extensions().get::<User>().cloned();
        let device = request.into_inner().device;

        let peer_id = devlog_sdk::distributed_id::gen_id().await.to_string();
        let region_code = current_region_code();
        let (region_code, signalling_route) = derive_signalling_region(region_code.as_deref());

        let (name, avatar_url, email, signalling_id) = match user {
            Some(user) => {
                let signalling_id = format!(
                    "{}_{}_{}",
                    devlog_sdk::distributed_id::gen_id().await,
                    user.order_id,
                    device.device_unique_key
                );

                (
                    Some(user.display_name.clone()),
                    user.avatar_url.unwrap_or_default(),
                    Some(user.email.clone()),
                    Some(signalling_id),
                )
            }
            None => {
                let avatar = self.app_service.random_avatar().await.unwrap_or_default();
                (Some(device.device_name.clone()), avatar, None, None)
            }
        };

        let peer = PeerMessage {
            peer_id,
            name,
            avatar_url,
            email,
            device,
            region_code: Some(region_code.clone()),
        };

        Ok(Response::new(GenPeerResponse {
            peer,
            signalling_id,
            region_code,
            signalling_route,
        }))
    }

    async fn get_region(&self, _request: Request<GetRegionRequest>) -> Result<Response<GetRegionResponse>, Status> {
        let region_code = current_region_code();
        let (region_code, signalling_route) = derive_signalling_region(region_code.as_deref());

        Ok(Response::new(GetRegionResponse {
            region_code,
            signalling_route,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::{derive_signalling_region, normalize_railway_region, normalize_signalling_route, resolve_region_code};
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

    #[test]
    fn derives_local_region_when_env_missing() {
        let (region_code, signalling_route) = derive_signalling_region(None);

        assert_eq!(region_code, "local");
        assert_eq!(signalling_route, "rpc-signalling-local");
    }

    #[test]
    fn derives_route_from_region_code() {
        let (region_code, signalling_route) = derive_signalling_region(Some("us-west"));

        assert_eq!(region_code, "us-west");
        assert_eq!(signalling_route, "rpc-signalling-us-west");
    }

    #[test]
    fn uses_railway_replica_region_as_fallback() {
        let region_code = resolve_region_code(None, Some("eu-west"));

        assert_eq!(region_code.as_deref(), Some("eu-west"));
    }

    #[test]
    fn prefers_explicit_bytover_region_code() {
        let region_code = resolve_region_code(Some("ap-southeast"), Some("eu-west"));

        assert_eq!(region_code.as_deref(), Some("ap-southeast"));
    }

    #[test]
    fn canonicalizes_provider_formatted_railway_region() {
        let region_code = normalize_railway_region(Some("europe-west4-drams3a"));

        assert_eq!(region_code.as_deref(), Some("europe"));
    }

    #[test]
    fn preserves_already_short_railway_region() {
        let region_code = normalize_railway_region(Some("europe"));

        assert_eq!(region_code.as_deref(), Some("europe"));
    }
}
