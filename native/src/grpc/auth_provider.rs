use std::str::FromStr;

use core_services::db::repository::abstraction::repository::Repository;
use tonic::metadata::MetadataValue;
use tonic::Request;

use shared::entities::session::SessionType;
use shared::persistence::session::{SessionId, SessionRepository};
use crate::grpc::errors::NativeGrpcErrors;

pub struct AuthProvider {
    pub session_repository: SessionRepository
}

impl AuthProvider {
    pub async fn with_auth<T>(&self, request: &mut Request<T>) -> Result<(), NativeGrpcErrors> {
        let session = self
            .session_repository
            .find_one(&SessionId {
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
