use crate::app_gateway::app_info::{AppInfoErrors, AppInfoService};
use crate::app_gateway::markov::{Markov, MarkovErrors};
use crate::entities::device_alias::DeviceAlias;
use crate::entities::p2p_session::P2PSession;
use crate::repositories::device_alias::DeviceAliasRepository;
use crate::repositories::p2p_session::P2PSessionRepository;
use core_services::db::repository::abstraction::errors::RepositoryError;
use rand::Rng;
use std::sync::Arc;

const MAX_ALIASES_PER_DEVICE: usize = 10;

#[derive(Debug, thiserror::Error)]
pub enum P2PTransferErrors {
    #[error("System error {0}")]
    SystemError(#[from] RepositoryError),
    #[error("Failed to generate alias {0}")]
    MarkovError(#[from] MarkovErrors),
    #[error("Application service error {0}")]
    ApplicationServiceError(#[from] AppInfoErrors),
    #[error("Alias not found for this device")]
    AliasNotFound,
    #[error("Alias generation failed after retries")]
    AliasGenerationFailed
}

pub struct P2PTransferService {
    pub p2p_repository: Arc<dyn P2PSessionRepository>,
    pub device_alias_repository: Arc<dyn DeviceAliasRepository>,
    pub app_service: Box<dyn AppInfoService>,
    pub markov_generator: Box<dyn Markov>
}

impl P2PTransferService {
    pub async fn create_user_device_session(
        &self,
        user_id: u64,
        device_id: u64,
        device_name: String,
        alias: String,
        signalling_key: String
    ) -> Result<P2PSession, P2PTransferErrors> {
        let existing_session = self.p2p_repository.find_by_alias(alias.clone()).await?;

        if let Some(session) = existing_session {
            let updated_session = P2PSession::from_db(
                session.session_id(),
                device_id,
                user_id,
                session.alias().to_string(),
                Some(device_name),
                signalling_key
            );

            let updated_session = self.p2p_repository.update_session(updated_session).await?;
            return Ok(updated_session);
        }

        let new_session = P2PSession::new(device_id, user_id, alias, Some(device_name), signalling_key).await;
        let created_session = self.p2p_repository.create_session(new_session).await?;

        Ok(created_session)
    }

    pub async fn get_or_create_aliases(&self, user_id: u64, device_id: u64) -> Result<Vec<String>, P2PTransferErrors> {
        let existing = self.device_alias_repository.find_by_user_and_device(user_id, device_id).await?;
        let existing_count = existing.len();

        if existing_count >= MAX_ALIASES_PER_DEVICE {
            return Ok(existing.into_iter().map(|a| a.alias().to_string()).collect());
        }

        let mut aliases: Vec<String> = existing.into_iter().map(|a| a.alias().to_string()).collect();
        let needed = MAX_ALIASES_PER_DEVICE - existing_count;

        for _ in 0..needed {
            if let Some(new_alias) = self.generate_unique_alias().await? {
                let device_alias = DeviceAlias::new(new_alias.clone(), user_id, device_id);
                self.device_alias_repository.create_alias(device_alias).await?;
                aliases.push(new_alias);
            }
        }

        Ok(aliases)
    }

    async fn generate_unique_alias(&self) -> Result<Option<String>, P2PTransferErrors> {
        let markov_alias = self.markov_generator.generate_name().await?;

        if !self.device_alias_repository.alias_exists(&markov_alias).await? {
            return Ok(Some(markov_alias));
        }

        for _ in 0..5 {
            let random_alias = generate_random_alias();
            if !self.device_alias_repository.alias_exists(&random_alias).await? {
                return Ok(Some(random_alias));
            }
        }

        Ok(None)
    }
}

fn generate_random_alias() -> String {
    let mut rng = rand::thread_rng();
    (0..10).map(|_| (b'a' + rng.gen_range(0..26)) as char).collect()
}
