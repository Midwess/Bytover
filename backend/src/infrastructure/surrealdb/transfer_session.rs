use core_services::db::repository::abstraction::errors::RepositoryError;
use core_services::db::repository::abstraction::table::Table;
use core_services::db::surrealdb::connection::SurrealDbConnection;
use core_services::db::surrealdb::repository::SurrealDbRepository;
use core_services::utils::pool::reponse::PoolResponse;
use core_services::utils::pool::request::PoolRequest;
use surreal_derive_plus::surreal_quote;
use surreal_devl::surreal_qr::RPath;

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

#[async_trait::async_trait]
impl TransferSessionRepository for TransferSessionSurrealdbRepository {
    async fn find_session_by_alias(&self, alias: String) -> Result<Option<TransferSession>, RepositoryError> {
        let db = self.db.retrieve().await.unwrap();
        let table_name = TransferSession::get_table();
        let session: Option<TransferSession> = db
            .query(surreal_quote!(r#"SELECT * FROM #table_name WHERE alias=#val(&alias)"#))
            .await?
            .take(RPath::Index(0))?;

        Ok(session)
    }
}
