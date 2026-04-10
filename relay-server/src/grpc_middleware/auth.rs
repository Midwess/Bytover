use base64::Engine;
use tonic::body::Body;
use tonic::codegen::http::Request;
use tonic::{async_trait, Status};
use tonic_middleware::RequestInterceptor;

#[derive(Clone)]
pub struct RelayAuthInterceptor {
    secret: String,
}

impl RelayAuthInterceptor {
    pub fn new() -> Self {
        let secret = std::env::var("RELAY_SERVER_SECRET").unwrap_or_else(|_| "supersecret".to_string());
        Self { secret }
    }
}

impl Default for RelayAuthInterceptor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl RequestInterceptor for RelayAuthInterceptor {
    async fn intercept(&self, req: Request<Body>) -> Result<Request<Body>, Status> {
        match req.headers().get("authorization") {
            Some(auth_header) => {
                let auth_str = auth_header.to_str().map_err(|_| Status::unauthenticated("Invalid authorization header encoding"))?;

                if let Some(credentials) = auth_str.strip_prefix("Basic ") {
                    let decoded = base64::engine::general_purpose::STANDARD
                        .decode(credentials)
                        .map_err(|_| Status::unauthenticated("Invalid base64 encoding"))?;

                    let decoded_str =
                        String::from_utf8(decoded).map_err(|_| Status::unauthenticated("Invalid credentials encoding"))?;

                    if let Some((_username, password)) = decoded_str.split_once(':') {
                        if password == self.secret {
                            return Ok(req);
                        }
                    }
                }

                Err(Status::unauthenticated("Invalid credentials"))
            }
            None => Err(Status::unauthenticated("Missing authorization header")),
        }
    }
}
