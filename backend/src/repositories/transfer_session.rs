use crate::cloud_storage::storage::CloudStorage;
use crate::entities::transfer_session::TransferSession;
use core_services::db::repository::abstraction::errors::RepositoryError;
use core_services::db::repository::abstraction::id::DbId;
use core_services::db::repository::abstraction::repository::Repository;
use core_services::db::repository::abstraction::table::Table;

#[derive(Clone, Default)]
pub struct TransferSessionId {
    pub user_order_id: Option<u64>,
    pub order_id: Option<u64>,
}

impl Table<TransferSessionId> for TransferSession {
    fn get_table() -> &'static str {
        "transfer_session"
    }

    fn id(&self) -> TransferSessionId {
        TransferSessionId {
            user_order_id: Some(self.user_order_id()),
            order_id: Some(self.order_id()),
        }
    }
}

impl DbId for TransferSessionId {
    type Table = TransferSession;
}

#[async_trait::async_trait]
pub trait TransferSessionRepository: Repository<TransferSession, TransferSessionId> {
    async fn find_session_by_alias(&self, alias: String) -> Result<Option<TransferSession>, RepositoryError>;
    async fn delete_stale_sessions(&self, cloud_storage: &dyn CloudStorage) -> Result<(), RepositoryError>;
}
