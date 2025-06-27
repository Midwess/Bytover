use core_services::db::redb::id::RedbId;
use core_services::db::redb::repository::RedbRepository;
use core_services::db::redb::table::RedbTable;
use core_services::db::repository::abstraction::errors::Resolve;
use core_services::db::repository::abstraction::repository::Repository;
use core_services::db::repository::abstraction::table::Table;
use core_services::utils::pool::reponse::PoolResponse;
use core_services::utils::pool::request::PoolRequest;
use redb::Database;
use shared::app::file_system::file::LocalResource;
use shared::app::repository::local_resource::{LocalResourceId, LocalResourceRepository};

use crate::repository::id::RedbIdWrapper;

pub struct LocalResourceRepositoryImpl {
    pub db: PoolRequest<Database>
}

impl RedbId for RedbIdWrapper<LocalResourceId> {
    fn lower_id(&self) -> Vec<Vec<u8>> {
        let code = bincode::serialize(&self.0.r#type).unwrap();
        let path = bincode::serialize(&self.0.path).unwrap();
        let id = bincode::serialize(&self.0.order_id).unwrap();
        vec![code, path, id]
    }
}

impl Table<RedbIdWrapper<LocalResourceId>> for LocalResource {
    fn get_table() -> &'static str {
        <Self as Table<LocalResourceId>>::get_table()
    }

    fn id(&self) -> RedbIdWrapper<LocalResourceId> {
        RedbIdWrapper(Table::id(self))
    }
}

impl RedbTable<RedbIdWrapper<LocalResourceId>> for LocalResource {}

#[async_trait::async_trait]
impl RedbRepository<LocalResource, RedbIdWrapper<LocalResourceId>> for LocalResourceRepositoryImpl {
    async fn get_db(&self) -> PoolResponse<Database> {
        self.db.retrieve().await.unwrap()
    }
}

#[async_trait::async_trait]
impl Repository<LocalResource, LocalResourceId> for LocalResourceRepositoryImpl {
    async fn create(&self, data: LocalResource) -> Resolve<LocalResource>
    where
        LocalResource: 'async_trait
    {
        RedbRepository::<LocalResource, RedbIdWrapper<LocalResourceId>>::create(self, data).await
    }

    async fn delete_one(&self, id: &LocalResourceId) -> Resolve<LocalResource> {
        RedbRepository::<LocalResource, RedbIdWrapper<LocalResourceId>>::delete_one(self, &RedbIdWrapper(id.clone())).await
    }

    async fn find_one(&self, id: &LocalResourceId) -> Resolve<Option<LocalResource>> {
        RedbRepository::<LocalResource, RedbIdWrapper<LocalResourceId>>::find_one(self, &RedbIdWrapper(id.clone())).await
    }

    async fn update_one(&self, data: LocalResource) -> Resolve<LocalResource> {
        RedbRepository::<LocalResource, RedbIdWrapper<LocalResourceId>>::update_one(self, data).await
    }

    async fn find_all(
        &self,
        from_id: Option<&LocalResourceId>,
        to_id: Option<&LocalResourceId>,
        count: Option<usize>
    ) -> Resolve<Vec<LocalResource>> {
        let to_id = to_id.map(|it| RedbIdWrapper(it.clone()));
        RedbRepository::<LocalResource, RedbIdWrapper<LocalResourceId>>::find_all(
            self,
            from_id.map(|it| RedbIdWrapper(it.clone())).as_ref(),
            to_id.as_ref(),
            count
        )
        .await
    }
}

#[async_trait::async_trait]
impl LocalResourceRepository for LocalResourceRepositoryImpl {}
