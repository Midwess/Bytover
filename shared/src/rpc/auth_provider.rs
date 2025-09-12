use anyhow::anyhow;
use std::str::FromStr;
use tonic::metadata::MetadataValue;
use tonic::Request;

use crate::app::repository::auth_session::{AuthSessionId, AuthSessionRepository};
use crate::entities::session::SessionType;
use crate::rpc::errors::RpcErrors;

pub struct AuthProvider {
    pub session_repository: &'static Box<dyn AuthSessionRepository>
}

impl AuthProvider {
    pub async fn with_auth<T>(&self, request: &mut Request<T>) -> Result<(), RpcErrors> {
        let repo = self.session_repository;
        let session = repo
            .find_one(&AuthSessionId {
                r#type: SessionType::Access
            })
            .await
            .map_err(|e| RpcErrors::AuthError(anyhow!("Failed to load authentication session {e:?}")))?;

        let Some(session) = session else {
            return Err(RpcErrors::AuthError(anyhow!("Session not found")));
        };

        if let Ok(token) = MetadataValue::from_str(&session.token.value) {
            request.metadata_mut().insert("authorization", token);
        }

        Ok(())
    }
}
