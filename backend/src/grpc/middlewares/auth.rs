use schema::devlog::auth_gateway::rpc::MeRequest;
use tonic::codegen::http::Request;
use tonic::metadata::MetadataValue;
use tonic::transport::Body;
use tonic::{async_trait, Status};
use tonic_middleware::RequestInterceptor;

use crate::di_container::DiContainer;

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
                let mut user_service = di_container.get_user_service().await?;
                let request_body = MeRequest { conditions: vec![] };

                let mut request = tonic::Request::new(request_body);
                let token_str = token.to_str().unwrap().to_owned();
                request.metadata_mut().insert("authorization", MetadataValue::try_from(token_str).unwrap());

                let user_info = user_service.me(request).await?;
                let user = user_info.into_inner().user;

                req.extensions_mut().insert(user);
                Ok(req)
            }
            None => Ok(req)
        }
    }
}
