use std::str::FromStr;

use core_services::db::repository::abstraction::repository::Repository;
use tonic::metadata::MetadataValue;
use tonic::Request;

use crate::entities::session::SessionType;
use crate::errors::NetworkError;
use crate::persistence::session::{SessionId, SessionRepository};

pub struct AuthProvider {
    pub session_repository: SessionRepository
}

impl AuthProvider {
    pub async fn with_auth<T>(&self, request: &mut Request<T>) -> Result<(), NetworkError> {
        let session = self
            .session_repository
            .find_one(&SessionId {
                r#type: SessionType::Access
            })
            .await
            .map_err(|e| NetworkError::Unauthorized(e.to_string()))?;

        log::info!("Session: {:?}", session);

        if session.is_none() {
            return Err(NetworkError::Unauthorized("Session not found".to_string()));
        }

        let token = session.unwrap().token;

        if let Ok(token) = MetadataValue::from_str(&token.value) {
            request.metadata_mut().insert("authorization", token);
        }

        Ok(())
    }
}
