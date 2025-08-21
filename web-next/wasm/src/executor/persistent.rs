use std::sync::Arc;
use shared::app::repository::auth_session::{AuthSessionId, AuthSessionRepository};
use shared::app::repository::local_resource::{LocalResourceId, LocalResourceRepository};
use shared::app::repository::transfer_session::{TransferSessionId, TransferSessionRepository};
use shared::executor::persistent::NativePersistent;

pub struct NativePersistentImpl {
    pub auth_session_repository: Box<dyn AuthSessionRepository>,
    pub local_resource_repository: Arc<dyn LocalResourceRepository>,
    pub transfer_session_repository: Box<dyn TransferSessionRepository>
}

#[async_trait::async_trait(?Send)]
impl NativePersistent for NativePersistentImpl {
    fn auth_session_repository(&self) -> &Box<dyn AuthSessionRepository> {
        &self.auth_session_repository
    }

    fn local_resource_repository(&self) -> &dyn LocalResourceRepository {
        &*self.local_resource_repository
    }

    fn transfer_session_repository(&self) -> &Box<dyn TransferSessionRepository> {
        &self.transfer_session_repository
    }
}
