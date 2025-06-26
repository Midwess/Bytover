use crate::app::file_system::file::LocalResource;
use crate::app::transfer::session::{TransferProgress, TransferSession, TransferType};
use crate::app::transfer::target::TransferTarget;
use core_services::db::repository::abstraction::errors::RepositoryError;
use core_services::db::repository::abstraction::id::DbId;
use core_services::db::repository::abstraction::repository::Repository;
use core_services::db::repository::abstraction::table::Table;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TransferSessionId {
    pub r#type: Option<TransferType>,
    pub target: Option<TransferTarget>,
    pub order_id: Option<u64>
}

#[async_trait::async_trait]
pub trait TransferSessionRepository: Repository<TransferSession, TransferSessionId> {
    async fn update_progresses(
        &self,
        order_id: u64,
        progresses: Vec<TransferProgress>
    ) -> Result<Option<TransferSession>, RepositoryError>;
    async fn update_resource(
        &self,
        session_id: TransferSessionId,
        resource: LocalResource
    ) -> Result<Option<TransferSession>, RepositoryError>;
}

impl Table<TransferSessionId> for TransferSession {
    fn get_table() -> &'static str {
        "transferSession"
    }

    fn id(&self) -> TransferSessionId {
        TransferSessionId {
            r#type: Some(self.transfer_type.clone()),
            target: Some(self.target.clone()),
            order_id: Some(self.order_id)
        }
    }
}

impl DbId for TransferSessionId {
    fn soft_deleted(&self) -> bool {
        false
    }

    fn soft_delete(&mut self) {}

    fn soft_restore(&mut self) {}
}
