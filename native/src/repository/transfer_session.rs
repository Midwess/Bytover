use crate::repository::id::RedbIdWrapper;
use bytes::Bytes;
use core_services::db::redb::id::RedbId;
use core_services::db::redb::repository::RedbRepository;
use core_services::db::redb::table::RedbTable;
use core_services::db::repository::abstraction::errors::{RepositoryError, Resolve};
use core_services::db::repository::abstraction::repository::Repository;
use core_services::db::repository::abstraction::table::Table;
use core_services::utils::pool::reponse::PoolResponse;
use core_services::utils::pool::request::PoolRequest;
use redb::Database;
use shared::app::file_system::file::LocalResource;
use shared::app::repository::transfer_session::{TransferSessionId, TransferSessionRepository};
use shared::app::transfer::session::{TransferProgress, TransferSession};

pub struct TransferSessionRepositoryImpl {
    pub db: PoolRequest<Database>
}

impl RedbId for RedbIdWrapper<TransferSessionId> {
    fn id(&self) -> Vec<Bytes> {
        let code = bincode::serialize(&self.0.r#type).unwrap();
        let target = bincode::serialize(&self.0.target).unwrap();
        let id = bincode::serialize(&self.0.order_id).unwrap();
        vec![
            Bytes::from(code),
            Bytes::from(target),
            Bytes::from(id),
        ]
    }
}

impl Table<RedbIdWrapper<TransferSessionId>> for TransferSession {
    fn get_table() -> &'static str {
        <Self as Table<TransferSessionId>>::get_table()
    }

    fn id(&self) -> RedbIdWrapper<TransferSessionId> {
        RedbIdWrapper(Table::id(self))
    }
}

impl RedbTable<RedbIdWrapper<TransferSessionId>> for TransferSession {}

#[async_trait::async_trait]
impl RedbRepository<TransferSession, RedbIdWrapper<TransferSessionId>> for TransferSessionRepositoryImpl {
    async fn get_db(&self) -> PoolResponse<Database> {
        self.db.retrieve().await.unwrap()
    }
}

#[async_trait::async_trait]
impl Repository<TransferSession, TransferSessionId> for TransferSessionRepositoryImpl {
    async fn create(&self, data: TransferSession) -> Resolve<TransferSession>
    where
        TransferSession: 'async_trait
    {
        RedbRepository::<TransferSession, RedbIdWrapper<TransferSessionId>>::create(self, data).await
    }

    async fn delete_one(&self, id: &TransferSessionId) -> Resolve<TransferSession> {
        RedbRepository::<TransferSession, RedbIdWrapper<TransferSessionId>>::delete_one(self, &RedbIdWrapper(id.clone())).await
    }

    async fn find_one(&self, id: &TransferSessionId) -> Resolve<Option<TransferSession>> {
        RedbRepository::<TransferSession, RedbIdWrapper<TransferSessionId>>::find_one(self, &RedbIdWrapper(id.clone())).await
    }

    async fn update_one(&self, data: TransferSession) -> Resolve<TransferSession> {
        RedbRepository::<TransferSession, RedbIdWrapper<TransferSessionId>>::update_one(self, data).await
    }

    async fn find_all(
        &self,
        from_id: Option<&TransferSessionId>,
        to_id: Option<&TransferSessionId>,
        count: Option<usize>
    ) -> Resolve<Vec<TransferSession>> {
        let to_id = to_id.map(|it| RedbIdWrapper(it.clone()));
        RedbRepository::<TransferSession, RedbIdWrapper<TransferSessionId>>::find_all(
            self,
            from_id.map(|it| RedbIdWrapper(it.clone())).as_ref(),
            to_id.as_ref(),
            count
        )
        .await
    }
}

#[async_trait::async_trait]
impl TransferSessionRepository for TransferSessionRepositoryImpl {
    async fn update_progresses(
        &self,
        order_id: u64,
        progresses: Vec<TransferProgress>
    ) -> Result<Option<TransferSession>, RepositoryError> {
        let id = TransferSessionId {
            order_id: Some(order_id),
            ..Default::default()
        };
        let session = RedbRepository::<TransferSession, RedbIdWrapper<TransferSessionId>>::find_one(
            self,
            &RedbIdWrapper(TransferSessionId {
                order_id: Some(order_id),
                ..Default::default()
            })
        )
        .await?;

        if let Some(session) = session {
            let mut session = session;
            session.progress = progresses;
            let result = RedbRepository::<TransferSession, RedbIdWrapper<TransferSessionId>>::update_one(self, session).await?;
            Ok(Some(result))
        } else {
            Ok(None)
        }
    }

    async fn update_resource(
        &self,
        session_id: TransferSessionId,
        resource: LocalResource
    ) -> Result<Option<TransferSession>, RepositoryError> {
        let session =
            RedbRepository::<TransferSession, RedbIdWrapper<TransferSessionId>>::find_one(self, &RedbIdWrapper(session_id.clone()))
                .await?;

        if let Some(mut session) = session {
            session.replace_resource(resource);
            let result = RedbRepository::<TransferSession, RedbIdWrapper<TransferSessionId>>::update_one(self, session).await?;
            Ok(Some(result))
        } else {
            Ok(None)
        }
    }
}
