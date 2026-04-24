use anyhow::anyhow;
use async_trait::async_trait;
use std::str::FromStr;
use tonic::metadata::MetadataValue;
use tonic::Request;

use crate::entities::session::SessionType;
use crate::protocol::rpc::errors::RpcErrors;
use crate::repository::auth_session::{AuthSessionId, AuthSessionRepository};

pub const APP_AUTHORIZATION_METADATA_KEY: &str = "x-app-authorization";

#[async_trait]
pub trait AppAuthTokenProvider: Send + Sync {
    async fn token(&self) -> Result<String, RpcErrors>;
}

pub struct EnvAppAuthTokenProvider;

const COMPILE_TIME_APP_AUTH_TOKEN: Option<&str> = option_env!("APP_AUTH_TOKEN");

#[async_trait]
impl AppAuthTokenProvider for EnvAppAuthTokenProvider {
    async fn token(&self) -> Result<String, RpcErrors> {
        match COMPILE_TIME_APP_AUTH_TOKEN {
            Some(value) if !value.is_empty() => Ok(value.to_owned()),
            _ => Err(RpcErrors::AuthError(anyhow!(
                "APP_AUTH_TOKEN was not set at build time; the desktop cannot authorize app-gateway payment calls"
            ))),
        }
    }
}

pub struct AuthProvider {
    pub session_repository: Box<dyn AuthSessionRepository>,
    pub app_auth_token: Box<dyn AppAuthTokenProvider>,
}

impl AuthProvider {
    pub async fn authorization_header(&self) -> Result<Option<String>, RpcErrors> {
        let session = self
            .session_repository
            .find_one(&AuthSessionId {
                r#type: SessionType::Access,
            })
            .await
            .map_err(|e| RpcErrors::AuthError(anyhow!("Failed to load authentication session {e:?}")))?;

        Ok(session.map(|session| session.token.value))
    }

    pub async fn with_auth<T>(&self, request: &mut Request<T>) -> Result<(), RpcErrors> {
        let Some(header_value) = self.authorization_header().await? else {
            return Err(RpcErrors::AuthError(anyhow!("You need to login first")));
        };

        if let Ok(token) = MetadataValue::from_str(&header_value) {
            request.metadata_mut().insert("authorization", token);
        }

        Ok(())
    }
}
