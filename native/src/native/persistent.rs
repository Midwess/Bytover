use shared::app::repository::auth_session::AuthSessionRepository;
use shared::app::repository::local_resource::LocalResourceRepository;
use shared::app::repository::transfer_session::TransferSessionRepository;
use shared::executor::persistent::NativePersistent;

pub struct NativePersistentImpl {
    pub auth_session_repository: Box<dyn AuthSessionRepository>,
    pub local_resource_repository: Box<dyn LocalResourceRepository>,
    pub transfer_session_repository: Box<dyn TransferSessionRepository>
}

#[async_trait::async_trait]
impl NativePersistent for NativePersistentImpl {
    fn auth_session_repository(&self) -> &Box<dyn AuthSessionRepository> {
        &self.auth_session_repository
    }

    fn local_resource_repository(&self) -> &Box<dyn LocalResourceRepository> {
        &self.local_resource_repository
    }

    fn transfer_session_repository(&self) -> &Box<dyn TransferSessionRepository> {
        &self.transfer_session_repository
    }
}
