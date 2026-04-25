use schema::devlog::bitbridge::Capabilities as CapabilitiesMsg;
use schema::devlog::app_gateway::rpc::MeRequest;
use tonic::body::Body;
use tonic::codegen::http::Request;
use tonic::metadata::MetadataValue;
use tonic::{async_trait, Status};
use tonic_middleware::RequestInterceptor;

use crate::app_gateway::plan::build_capabilities_msg;
use crate::di_container::DiContainer;
use crate::entities::user_capabilities::UserCapabilities;
use crate::repositories::user_capabilities::UserCapabilitiesRepository;
use crate::user::Token;

#[derive(Clone)]
pub struct AuthInterceptor {}

impl Default for AuthInterceptor {
    fn default() -> Self {
        Self::new()
    }
}

impl AuthInterceptor {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl RequestInterceptor for AuthInterceptor {
    async fn intercept(&self, mut req: Request<Body>) -> Result<Request<Body>, Status> {
        match req.headers().get("authorization") {
            Some(token) => {
                let di_container = DiContainer::instance().await;
                let mut user_service = di_container.get_user_service().await.map_err(|e| Status::internal(e.to_string()))?;
                let request_body = MeRequest { conditions: vec![] };

                let mut request = tonic::Request::new(request_body);
                let token_str = token.to_str().map_err(|_| Status::invalid_argument("Invalid token"))?.to_owned();
                request.metadata_mut().insert("authorization", MetadataValue::try_from(token_str.clone()).unwrap());

                let user_info = user_service.me(request).await?;
                let user_info = user_info.into_inner();
                let user = user_info.user;
                let app = user_info.app;
                let device = user_info.device;

                let caps_repo = di_container.get_user_capabilities_repository().await;
                let caps_row = caps_repo
                    .find_or_create_default(user.order_id, &device.unique_key)
                    .await
                    .map_err(|e| Status::internal(e.to_string()))?;
                let caps_msg = build_capabilities_msg(&caps_row);

                req.extensions_mut().insert::<UserCapabilities>(caps_row);
                req.extensions_mut().insert::<CapabilitiesMsg>(caps_msg);
                req.extensions_mut().insert(device);
                req.extensions_mut().insert(user);
                req.extensions_mut().insert(app);
                req.extensions_mut().insert(Token(token_str));
                Ok(req)
            }
            None => Ok(req),
        }
    }
}
