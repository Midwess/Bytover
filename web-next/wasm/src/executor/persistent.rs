use shared::app::repository::auth_session::AuthSessionRepository;
use shared::app::repository::local_resource::LocalResourceRepository;
use shared::app::repository::transfer_session::TransferSessionRepository;
use shared::executor::persistent::NativePersistent;
use std::sync::Arc;
use shared::app::operations::persistent::{LocalResourcePersistentOperation, LocalResourcePersistentOperationOutput, PersistentOperation, PersistentOperationOutput};

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
        match effect {
            PersistentOperation::LocalResource(LocalResourcePersistentOperation::Remove(_)) => {
                return PersistentOperationOutput::LocalResource(LocalResourcePersistentOperationOutput::Removed)
            }
            _ => {}
        };

        NativePersistent::default_handle(self, effect).await
    }
}
