use core_services::db::redb::id::RedbId;
use core_services::db::redb::repository::RedbRepository;
use core_services::db::redb::table::RedbTable;
use core_services::db::repository::abstraction::errors::Resolve;
use core_services::db::repository::abstraction::repository::Repository;
use core_services::db::repository::abstraction::table::Table;
use core_services::utils::pool::reponse::PoolResponse;
use core_services::utils::pool::request::PoolRequest;
use redb::Database;
use shared::app::repository::auth_session::{AuthSessionId, AuthSessionRepository};
use shared::entities::session::Session;

use crate::repository::id::RedbIdWrapper;

pub struct AuthSessionRepositoryImpl {
    pub db: PoolRequest<Database>
}

impl RedbId for RedbIdWrapper<AuthSessionId> {
    fn lower_id(&self) -> Vec<Vec<u8>> {
        let code = bincode::serialize(&self.0.r#type).unwrap();
        vec![code]
    }
}

impl Table<RedbIdWrapper<AuthSessionId>> for Session {
    fn get_table() -> &'static str {
        <Self as Table<AuthSessionId>>::get_table()
    }

    fn id(&self) -> RedbIdWrapper<AuthSessionId> {
        RedbIdWrapper(Table::id(self))
    }
}

impl RedbTable<RedbIdWrapper<AuthSessionId>> for Session {}

#[async_trait::async_trait]
impl RedbRepository<Session, RedbIdWrapper<AuthSessionId>> for AuthSessionRepositoryImpl {
    async fn get_db(&self) -> PoolResponse<Database> {
        self.db.retrieve().await.unwrap()
    }
}

#[async_trait::async_trait]
impl Repository<Session, AuthSessionId> for AuthSessionRepositoryImpl {
    async fn create(&self, data: Session) -> Resolve<Session>
    where
        Session: 'async_trait
    {
        RedbRepository::<Session, RedbIdWrapper<AuthSessionId>>::create(self, data).await
    }

    async fn delete_one(&self, id: &AuthSessionId) -> Resolve<Session> {
        RedbRepository::<Session, RedbIdWrapper<AuthSessionId>>::delete_one(self, &RedbIdWrapper(id.clone())).await
    }

    async fn find_one(&self, id: &AuthSessionId) -> Resolve<Option<Session>> {
        RedbRepository::<Session, RedbIdWrapper<AuthSessionId>>::find_one(self, &RedbIdWrapper(id.clone())).await
    }

    async fn update_one(&self, data: Session) -> Resolve<Session> {
        RedbRepository::<Session, RedbIdWrapper<AuthSessionId>>::update_one(self, data).await
    }

    async fn find_all(
        &self,
        from_id: Option<&AuthSessionId>,
        to_id: Option<&AuthSessionId>,
        count: Option<usize>
    ) -> Resolve<Vec<Session>> {
        let to_id = to_id.map(|it| RedbIdWrapper(it.clone()));
        RedbRepository::<Session, RedbIdWrapper<AuthSessionId>>::find_all(
            self,
            from_id.map(|it| RedbIdWrapper(it.clone())).as_ref(),
            to_id.as_ref(),
            count
        )
        .await
    }
}

#[async_trait::async_trait]
impl AuthSessionRepository for AuthSessionRepositoryImpl {}
