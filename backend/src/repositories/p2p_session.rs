use crate::entities::p2p_session::P2PSession;
use core_services::db::repository::abstraction::errors::RepositoryError;
use core_services::db::repository::abstraction::id::DbId;
use core_services::db::repository::abstraction::repository::Repository;
use core_services::db::repository::abstraction::table::Table;

#[derive(Clone, Default)]
pub struct P2PSessionId {
    pub session_id: Option<u64>,
}

impl Table<P2PSessionId> for P2PSession {
    fn get_table() -> &'static str {
        "p2pSession"
    }

    fn id(&self) -> P2PSessionId {
        P2PSessionId {
            session_id: Some(self.session_id()),
        }
    }
}

impl DbId for P2PSessionId {
    type Table = P2PSession;
}

#[async_trait::async_trait]
pub trait P2PSessionRepository: Repository<P2PSession, P2PSessionId> {
    async fn find_by_user_id_and_device_id(
        &self,
        user_id: u64,
        device_id: u64,
    ) -> Result<Option<P2PSession>, RepositoryError>;

    async fn find_by_alias(&self, alias: String) -> Result<Option<P2PSession>, RepositoryError>;

    async fn create_session(&self, session: P2PSession) -> Result<P2PSession, RepositoryError>;

    async fn update_session(&self, session: P2PSession) -> Result<P2PSession, RepositoryError>;
}
