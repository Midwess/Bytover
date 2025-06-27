use std::str::FromStr;

use tonic::metadata::MetadataValue;
use tonic::Request;

use crate::grpc::errors::NativeGrpcErrors;
use shared::app::repository::auth_session::{AuthSessionId, AuthSessionRepository};
use shared::entities::session::SessionType;

pub struct AuthProvider {
    pub session_repository: Box<dyn AuthSessionRepository>
}

impl AuthProvider {
    pub async fn with_auth<T>(&self, request: &mut Request<T>) -> Result<(), NativeGrpcErrors> {
        let session = self
            .session_repository
            .find_one(&AuthSessionId {
                r#type: SessionType::Access
            })
            .await
            .map_err(|e| NativeGrpcErrors::Unauthorized(e.to_string()))?;

        if session.is_none() {
            return Err(NativeGrpcErrors::Unauthorized("Session not found".to_string()));
        }

        let token = session.unwrap().token;

        if let Ok(token) = MetadataValue::from_str(&token.value) {
            request.metadata_mut().insert("authorization", token);
        }

        Ok(())
    }
}
