use crate::repository::id::RedbIdWrapper;
use core_services::db::redb::id::RedbId;
use core_services::db::redb::repository::RedbRepository;
use core_services::db::redb::table::RedbTable;
use core_services::db::repository::abstraction::errors::Resolve;
use core_services::db::repository::abstraction::repository::Repository;
use core_services::db::repository::abstraction::table::Table;
use core_services::utils::pool::reponse::PoolResponse;
use core_services::utils::pool::request::PoolRequest;
use redb::Database;
use shared::entities::user::User;
use shared::repository::user::{UserId, UserRepository};

pub struct UserRepositoryImpl {
    pub db: PoolRequest<Database>
}

impl RedbId for RedbIdWrapper<UserId> {
    fn lower_id(&self) -> Vec<Vec<u8>> {
        let email = bincode::serialize(&self.0.email).unwrap();
        vec![email]
    }
}

impl Table<RedbIdWrapper<UserId>> for User {
    fn get_table() -> &'static str {
        <Self as Table<UserId>>::get_table()
    }

    fn id(&self) -> RedbIdWrapper<UserId> {
        RedbIdWrapper(Table::id(self))
    }
}

impl RedbTable<RedbIdWrapper<UserId>> for User {}

#[async_trait::async_trait]
impl RedbRepository<User, RedbIdWrapper<UserId>> for UserRepositoryImpl {
    async fn get_db(&self) -> PoolResponse<Database> {
        self.db.retrieve().await.unwrap()
    }
}

#[async_trait::async_trait]
impl Repository<User, UserId> for UserRepositoryImpl {
    async fn create(&self, data: User) -> Resolve<User>
    where
        User: 'async_trait
    {
        RedbRepository::<User, RedbIdWrapper<UserId>>::create(self, data).await
    }

    async fn delete_one(&self, id: &UserId) -> Resolve<User> {
        RedbRepository::<User, RedbIdWrapper<UserId>>::delete_one(self, &RedbIdWrapper(id.clone())).await
    }

    async fn find_one(&self, id: &UserId) -> Resolve<Option<User>> {
        RedbRepository::<User, RedbIdWrapper<UserId>>::find_one(self, &RedbIdWrapper(id.clone())).await
    }

    async fn update_one(&self, data: User) -> Resolve<User> {
        RedbRepository::<User, RedbIdWrapper<UserId>>::update_one(self, data).await
    }

    async fn find_all(&self, from_id: Option<&UserId>, to_id: Option<&UserId>, count: Option<usize>) -> Resolve<Vec<User>> {
        let to_id = to_id.map(|it| RedbIdWrapper(it.clone()));
        RedbRepository::<User, RedbIdWrapper<UserId>>::find_all(
            self,
            from_id.map(|it| RedbIdWrapper(it.clone())).as_ref(),
            to_id.as_ref(),
            count
        )
        .await
    }
}

#[async_trait::async_trait]
impl UserRepository for UserRepositoryImpl {}
