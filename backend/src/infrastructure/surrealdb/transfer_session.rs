use core_services::db::remote_surrealdb::SurrealDbConnection;
use core_services::db::repository::abstraction::repository::SurrealDbRepository;
use core_services::utils::pool::reponse::PoolResponse;
use core_services::utils::pool::request::PoolRequest;

use crate::entities::transfer_session::TransferSession;
use crate::repositories::transfer_session::{TransferSessionId, TransferSessionRepository};

pub struct TransferSessionSurrealdbRepository {
    pub db: PoolRequest<SurrealDbConnection>
}

#[async_trait::async_trait]
impl SurrealDbRepository<TransferSession, TransferSessionId> for TransferSessionSurrealdbRepository {
    async fn get_db(&self) -> PoolResponse<SurrealDbConnection> {
        self.db.retrieve().await.unwrap()
    }
}

impl TransferSessionRepository for TransferSessionSurrealdbRepository {}
