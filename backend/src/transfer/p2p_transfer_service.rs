use crate::app_gateway::app_info::{AppInfoErrors, AppInfoService};
use crate::app_gateway::markov::{Markov, MarkovErrors};
use crate::entities::p2p_session::P2PSession;
use crate::repositories::p2p_session::P2PSessionRepository;
use core_services::db::repository::abstraction::errors::RepositoryError;
use std::sync::Arc;

#[derive(Debug, thiserror::Error)]
pub enum P2PTransferErrors {
    #[error("System error {0}")]
    SystemError(#[from] RepositoryError),
    #[error("Failed to generate alias {0}")]
    MarkovError(#[from] MarkovErrors),
    #[error("Application service error {0}")]
    ApplicationServiceError(#[from] AppInfoErrors),
}

pub struct P2PTransferService {
    pub p2p_repository: Arc<dyn P2PSessionRepository>,
    pub app_service: Box<dyn AppInfoService>,
    pub markov_generator: Box<dyn Markov>,
}

impl P2PTransferService {
    pub async fn create_user_device_session(
        &self,
        user_id: u64,
        device_id: u64,
        password_protected: bool,
    ) -> Result<P2PSession, P2PTransferErrors> {
        // Try to find existing session for this user and device
        let existing_session = self
            .p2p_repository
            .find_by_user_id_and_device_id(user_id, device_id)
            .await?;

        if let Some(session) = existing_session {
            let updated_session = P2PSession::from_db(
                session.session_id(),
                device_id,
                user_id,
                session.alias().to_string(),
                password_protected,
            );

            let updated_session = self.p2p_repository.update_session(updated_session).await?;
            return Ok(updated_session);
        }

        let alias = self.markov_generator.generate_name().await?;

        let new_session = P2PSession::new(
            device_id,
            user_id,
            alias,
            password_protected,
        )
        .await;

        let created_session = self.p2p_repository.create_session(new_session).await?;

        Ok(created_session)
    }
}
