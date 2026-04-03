use std::future::{ready, Ready};
use std::rc::Rc;
use actix_web::body::EitherBody;
use actix_web::dev::{Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::{web, Error, HttpMessage, HttpResponse};
use futures_util::future::LocalBoxFuture;
use crate::app_gateway::client::{AppGatewayClient, AuthError};

pub struct Auth;

impl<S, B> Transform<S, ServiceRequest> for Auth
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = AuthMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(AuthMiddleware {
            service: Rc::new(service),
        }))
    }
}

pub struct AuthMiddleware<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for AuthMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&self, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let auth_header = req.headers().get("authorization").cloned();
        let client = req.app_data::<web::Data<AppGatewayClient>>().cloned();
        let peer_addr = req
            .connection_info()
            .realip_remote_addr()
            .unwrap_or("0.0.0.0")
            .to_string();
        
        let service = self.service.clone();

        Box::pin(async move {
            let client = match client {
                Some(c) => c,
                None => {
                    log::error!("AppGatewayClient not found in app_data");
                    return Err(actix_web::error::ErrorInternalServerError("Internal server error"));
                }
            };

            let token = match auth_header {
                Some(auth_header) => {
                    let auth_str = auth_header.to_str().unwrap_or("");
                    if let Some(token) = auth_str.strip_prefix("Bearer ") {
                        token.to_string()
                    } else {
                        log::warn!("Invalid authorization header format from {}", peer_addr);
                        return Ok(req.into_response(
                            HttpResponse::Unauthorized()
                                .json(serde_json::json!({
                                    "error": "Invalid authorization header format"
                                }))
                                .map_into_right_body(),
                        ));
                    }
                }
                None => {
                    log::warn!("Missing authorization header from {}", peer_addr);
                    return Ok(req.into_response(
                        HttpResponse::Unauthorized()
                            .json(serde_json::json!({
                                "error": "Missing authorization header"
                            }))
                            .map_into_right_body(),
                    ));
                }
            };

            match client.validate_token(&token).await {
                Ok(auth_context) => {
                    // Inject auth context into request extensions
                    req.extensions_mut().insert(auth_context);
                    service.call(req).await.map(|res| res.map_into_left_body())
                }
                Err(e) => {
                    let (status, message) = match e {
                        AuthError::GrpcError(_) => {
                            (actix_web::http::StatusCode::SERVICE_UNAVAILABLE, "Authentication service unavailable")
                        }
                        AuthError::InvalidToken(_) => {
                            (actix_web::http::StatusCode::UNAUTHORIZED, "Invalid token")
                        }
                        AuthError::MissingToken => {
                            (actix_web::http::StatusCode::UNAUTHORIZED, "Missing token")
                        }
                    };
                    log::warn!("Auth failed from {}: {}", peer_addr, e);
                    Ok(req.into_response(
                        HttpResponse::build(status)
                            .json(serde_json::json!({
                                "error": message
                            }))
                            .map_into_right_body(),
                    ))
                }
            }
        })
    }
}
