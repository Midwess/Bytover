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
use shared::entities::shelf::Shelf;
use shared::repository::errors::PersistenceError;
use shared::repository::shelf::{ShelfId, ShelfRepository};

pub struct ShelfRepositoryImpl {
    pub db: PoolRequest<Database>
}

impl RedbId for RedbIdWrapper<ShelfId> {
    fn lower_id(&self) -> Vec<Vec<u8>> {
        let id = bincode::serialize(&self.0.id).unwrap();
        vec![id]
    }
}

impl Table<RedbIdWrapper<ShelfId>> for Shelf {
    fn get_table() -> &'static str {
        <Self as Table<ShelfId>>::get_table()
    }

    fn id(&self) -> RedbIdWrapper<ShelfId> {
        RedbIdWrapper(Table::id(self))
    }
}

impl RedbTable<RedbIdWrapper<ShelfId>> for Shelf {}

#[async_trait::async_trait]
impl RedbRepository<Shelf, RedbIdWrapper<ShelfId>> for ShelfRepositoryImpl {
    async fn get_db(&self) -> PoolResponse<Database> {
        self.db.retrieve().await.unwrap()
    }
}

#[async_trait::async_trait]
impl Repository<Shelf, ShelfId> for ShelfRepositoryImpl {
    async fn create(&self, data: Shelf) -> Resolve<Shelf>
    where
        Shelf: 'async_trait
    {
        RedbRepository::<Shelf, RedbIdWrapper<ShelfId>>::create(self, data).await
    }

    async fn delete_one(&self, id: &ShelfId) -> Resolve<Shelf> {
        RedbRepository::<Shelf, RedbIdWrapper<ShelfId>>::delete_one(self, &RedbIdWrapper(id.clone())).await
    }

    async fn find_one(&self, id: &ShelfId) -> Resolve<Option<Shelf>> {
        RedbRepository::<Shelf, RedbIdWrapper<ShelfId>>::find_one(self, &RedbIdWrapper(id.clone())).await
    }

    async fn update_one(&self, data: Shelf) -> Resolve<Shelf> {
        RedbRepository::<Shelf, RedbIdWrapper<ShelfId>>::update_one(self, data).await
    }

    async fn find_all(&self, from_id: Option<&ShelfId>, to_id: Option<&ShelfId>, count: Option<usize>) -> Resolve<Vec<Shelf>> {
        let to_id = to_id.map(|it| RedbIdWrapper(it.clone()));
        RedbRepository::<Shelf, RedbIdWrapper<ShelfId>>::find_all(
            self,
            from_id.map(|it| RedbIdWrapper(it.clone())).as_ref(),
            to_id.as_ref(),
            count
        )
        .await
    }
}

#[async_trait::async_trait]
impl ShelfRepository for ShelfRepositoryImpl {
    async fn load_all(&self, limit: Option<usize>) -> Result<Vec<Shelf>, PersistenceError> {
        let shelves = RedbRepository::find_all(self, None, None, limit).await?;
        Ok(shelves)
    }

    async fn add(&self, shelf: Shelf) -> Result<Shelf, PersistenceError> {
        let shelf = Repository::<Shelf, ShelfId>::create(self, shelf).await?;
        Ok(shelf)
    }

    async fn remove(&self, id: u64) -> Result<bool, PersistenceError> {
        let shelf_id = ShelfId { id: Some(id) };
        let deleted = Repository::<Shelf, ShelfId>::delete_one(self, &shelf_id).await?;
        Ok(deleted.id == id)
    }
}
