use shared::app::operations::persistent::{
    LocalResourcePersistentOperation,
    LocalResourcePersistentOperationOutput,
    PersistentOperation,
    PersistentOperationOutput
};
use shared::repository::auth_session::AuthSessionRepository;
use shared::repository::local_resource::LocalResourceRepository;
use shared::repository::transfer_session::TransferSessionRepository;
use shared::shell::executor::persistent::NativePersistent;
use std::sync::Arc;

pub struct NativePersistentImpl {
    pub auth_session_repository: Box<dyn AuthSessionRepository>,
    pub local_resource_repository: Arc<dyn LocalResourceRepository>,
    pub transfer_session_repository: Arc<dyn TransferSessionRepository>
}

#[async_trait::async_trait(?Send)]
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

    async fn handle(&self, effect: PersistentOperation) -> PersistentOperationOutput {
        if let PersistentOperation::LocalResource(LocalResourcePersistentOperation::Remove(_)) = effect {
            return PersistentOperationOutput::LocalResource(LocalResourcePersistentOperationOutput::Removed)
        };

        NativePersistent::default_handle(self, effect).await
    }
}
