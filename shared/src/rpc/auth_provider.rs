use anyhow::anyhow;
use std::str::FromStr;
use tonic::metadata::MetadataValue;
use tonic::Request;

use crate::app::repository::auth_session::{AuthSessionId, AuthSessionRepository};
use crate::entities::session::SessionType;
use crate::rpc::errors::RpcErrors;

pub struct AuthProvider {
    pub session_repository: Box<dyn AuthSessionRepository>
}

impl AuthProvider {
    pub async fn with_auth<T>(&self, request: &mut Request<T>) -> Result<(), RpcErrors> {
        let session = self
            .session_repository
            .find_one(&AuthSessionId {
                r#type: SessionType::Access
            })
            .await
            .map_err(|e| RpcErrors::AuthError(anyhow!("Failed to load authentication session {e:?}")))?;

        if session.is_none() {
            return Err(RpcErrors::AuthError(anyhow!("Session not found")));
        }

        let token = session.unwrap().token;

        if let Ok(token) = MetadataValue::from_str(&token.value) {
            request.metadata_mut().insert("authorization", token);
        }

        Ok(())
    }
}
