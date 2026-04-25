use crate::entities::user_capabilities::UserCapabilities;
use core_services::db::repository::abstraction::errors::RepositoryError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IncrementOutcome {
    pub new_bytes_used: u64,
    pub cap_crossed: bool,
    pub lifetime_cap: u64,
}

#[async_trait::async_trait]
pub trait UserCapabilitiesRepository: Send + Sync {
    async fn find_or_create_default(
        &self,
        user_order_id: u64,
        device_unique_key: &str,
    ) -> Result<UserCapabilities, RepositoryError>;

    async fn increment_bytes_used(&self, user_order_id: u64, delta: u64) -> Result<IncrementOutcome, RepositoryError>;

    async fn upgrade_to_paid(&self, user_order_id: u64) -> Result<UserCapabilities, RepositoryError>;
}
