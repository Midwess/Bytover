use shared::repository::auth_session::AuthSessionRepository;
use shared::repository::local_resource::LocalResourceRepository;
use shared::repository::shelf::ShelfRepository;
use shared::repository::transfer_session::TransferSessionRepository;
use shared::shell::executor::persistent::NativePersistent;

pub struct NativePersistentImpl {
    pub auth_session_repository: Box<dyn AuthSessionRepository>,
    pub local_resource_repository: Box<dyn LocalResourceRepository>,
    pub transfer_session_repository: Box<dyn TransferSessionRepository>,
    pub shelf_repository: Box<dyn ShelfRepository>
}

#[async_trait::async_trait]
impl NativePersistent for NativePersistentImpl {
    fn auth_session_repository(&self) -> &Box<dyn AuthSessionRepository> {
        &self.auth_session_repository
    }

    fn local_resource_repository(&self) -> &dyn LocalResourceRepository {
        &*self.local_resource_repository
    }

    fn transfer_session_repository(&self) -> &dyn TransferSessionRepository {
        &*self.transfer_session_repository
    }

    fn shelf_repository(&self) -> &dyn ShelfRepository {
        &*self.shelf_repository
    }
}
