use crate::repository::id::IdbIdWrapper;
use core_services::db::idb::id::IdbId;
use core_services::db::idb::repository::IdbRepository;
use core_services::db::idb::table::IdbTable;
use core_services::db::repository::abstraction::errors::{RepositoryError, Resolve};
use core_services::db::repository::abstraction::repository::Repository;
use core_services::db::repository::abstraction::table::Table;
use core_services::utils::never_send::NeverSend;
use core_services::utils::pool::reponse::PoolResponse;
use core_services::utils::pool::request::PoolRequest;
use devlog_sdk::distributed_id::gen_id;
use idb::Database;
use shared::entities::shelf::Shelf;
use shared::repository::errors::PersistenceError;
use shared::repository::shelf::{ShelfId, ShelfRepository};
use wasm_bindgen::JsValue;

pub struct ShelfRepositoryImpl {
    pub db: PoolRequest<NeverSend<Database>>
}

impl IdbId for IdbIdWrapper<ShelfId> {
    fn deserialize(value: JsValue) -> Result<Self, RepositoryError> {
        let mut json: serde_json::Value = serde_wasm_bindgen::from_value(value)?;
        let table_name = "shelf";
        if !json.is_array() {
            return Err(RepositoryError::Conflict(
                table_name.to_owned(),
                "The id must be an array of primitive types".to_owned()
            ));
        }

        let Some(json_array) = json.as_array_mut() else {
            return Err(RepositoryError::Conflict(
                table_name.to_owned(),
                "The id must be an array of primitive types".to_owned()
            ));
        };

        Ok(IdbIdWrapper(ShelfId {
            id: json_array.first().and_then(|it| it.as_str().and_then(|it| it.parse().ok()))
        }))
    }
}

impl Table<IdbIdWrapper<ShelfId>> for Shelf {
    fn get_table() -> &'static str {
        <Self as Table<ShelfId>>::get_table()
    }

    fn id(&self) -> IdbIdWrapper<ShelfId> {
        IdbIdWrapper(Table::id(self))
    }
}

impl IdbTable<IdbIdWrapper<ShelfId>> for Shelf {}

#[async_trait::async_trait(?Send)]
impl IdbRepository<Shelf, IdbIdWrapper<ShelfId>> for ShelfRepositoryImpl {
    async fn get_db(&self) -> PoolResponse<NeverSend<Database>> {
        self.db.retrieve().await.unwrap()
    }
}

#[async_trait::async_trait(?Send)]
impl Repository<Shelf, ShelfId> for ShelfRepositoryImpl {
    async fn create(&self, data: Shelf) -> Resolve<Shelf>
    where
        Shelf: 'async_trait
    {
        IdbRepository::<Shelf, IdbIdWrapper<ShelfId>>::create(self, data).await
    }

    async fn find_one(&self, id: &ShelfId) -> Resolve<Option<Shelf>> {
        IdbRepository::<Shelf, IdbIdWrapper<ShelfId>>::find_one(self, &IdbIdWrapper(id.clone())).await
    }

    async fn find_all(&self, from_id: Option<&ShelfId>, to_id: Option<&ShelfId>, count: Option<usize>) -> Resolve<Vec<Shelf>> {
        let to_id = to_id.map(|it| IdbIdWrapper(it.clone()));
        IdbRepository::<Shelf, IdbIdWrapper<ShelfId>>::find_all(
            self,
            from_id.map(|it| IdbIdWrapper(it.clone())).as_ref(),
            to_id.as_ref(),
            count
        )
        .await
    }

    async fn delete_one(&self, id: &ShelfId) -> Resolve<Shelf> {
        IdbRepository::<Shelf, IdbIdWrapper<ShelfId>>::delete_one(self, &IdbIdWrapper(id.clone())).await
    }

    async fn update_one(&self, data: Shelf) -> Resolve<Shelf> {
        IdbRepository::<Shelf, IdbIdWrapper<ShelfId>>::update_one(self, data).await
    }
}

#[async_trait::async_trait(?Send)]
impl ShelfRepository for ShelfRepositoryImpl {
    async fn load_all(&self) -> Result<Vec<Shelf>, PersistenceError> {
        let shelves = IdbRepository::find_all(self, None, None, None).await?;
        Ok(shelves)
    }

    async fn add(&self, mut shelf: Shelf) -> Result<Shelf, PersistenceError> {
        shelf.id = gen_id().await;
        let shelf = Repository::<Shelf, ShelfId>::create(self, shelf).await?;
        Ok(shelf)
    }

    async fn remove(&self, id: u64) -> Result<bool, PersistenceError> {
        let shelf_id = ShelfId { id: Some(id) };
        let deleted = Repository::<Shelf, ShelfId>::delete_one(self, &shelf_id).await?;
        Ok(deleted.id == id)
    }
}
