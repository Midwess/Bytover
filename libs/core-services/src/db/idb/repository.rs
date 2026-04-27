use crate::db::idb::id::IdbId;
use crate::db::idb::table::IdbTable;
use crate::db::repository::abstraction::errors::{RepositoryError, Resolve};
use crate::db::repository::abstraction::repository::Repository;
use crate::utils::never_send::NeverSend;
use crate::utils::pool::reponse::PoolResponse;
use idb::{Database, KeyRange, Query, TransactionMode};

#[async_trait::async_trait(?Send)]
pub trait IdbRepository<T, I>: Send + Sync + Repository<T, I>
where
    T: Send + Sync + IdbTable<I>,
    I: Send + Sync + IdbId
{
    async fn get_db(&self) -> PoolResponse<NeverSend<Database>>;

    async fn create(&self, item: T) -> Resolve<T>
    where
        T: 'async_trait
    {
        let table_name = T::get_table();

        let db = self.get_db().await;

        let transaction = db.transaction(&[table_name], TransactionMode::ReadWrite)?;
        let store = transaction.object_store(table_name)?;
        let id = IdbTable::id(&item);
        let key = IdbId::serialize(&id)?;
        let value = IdbTable::serialize(&item)?;

        let inserted_key = store.add(&value, Some(&key))?.await?;
        let Some(inserted_value) = store.get(inserted_key)?.await? else {
            return Err(RepositoryError::NotFound(table_name.to_owned(), format!("{:?}", id)));
        };

        let inserted_item: T = IdbTable::deserialize(inserted_value)?;

        Ok(inserted_item)
    }

    async fn find_one(&self, id: &I) -> Resolve<Option<T>> {
        let result = IdbRepository::find_all_with_keys(self, Some(id), None, None).await?;
        if result.is_empty() {
            return Ok(None);
        }

        let mut result = result.into_iter().filter(|it| IdbId::equals(id, &it.0).unwrap_or(false)).collect::<Vec<_>>();

        if result.is_empty() {
            return Ok(None);
        }

        let (key, value) = result.remove(0);

        if !IdbId::equals(id, &key)? {
            return Ok(None);
        }

        Ok(Some(value))
    }

    async fn find_all(&self, from: Option<&I>, to: Option<&I>, limit: Option<usize>) -> Resolve<Vec<T>> {
        let result = IdbRepository::find_all_with_keys(self, from, to, limit).await?;
        Ok(result.into_iter().map(|(_id, item)| item).collect())
    }

    async fn find_all_with_keys(&self, from: Option<&I>, to: Option<&I>, limit: Option<usize>) -> Resolve<Vec<(I, T)>> {
        let table_name = T::get_table();
        let db = self.get_db().await;

        let from_id = match from {
            Some(id) => {
                let id = IdbId::into_query_value(id)?;
                Some(id)
            }
            None => None
        };

        let to_id = match to {
            Some(to_id) => {
                let to_id = IdbId::into_query_value(to_id)?;
                Some(to_id)
            }
            None => None
        };

        let range_query = match (from_id, to_id) {
            (Some(from_id), Some(to_id)) => {
                Some(Query::KeyRange(KeyRange::bound(&from_id, &to_id, Some(false), Some(false))?))
            }
            (Some(from_id), None) => Some(Query::KeyRange(KeyRange::lower_bound(&from_id, Some(false))?)),
            (None, Some(to_id)) => Some(Query::KeyRange(KeyRange::upper_bound(&to_id, Some(false))?)),
            (None, None) => None
        };

        let transaction = db.transaction(&[table_name], TransactionMode::ReadOnly)?;
        let store = transaction.object_store(table_name)?;
        let values = store.get_all(range_query.clone(), limit.map(|it| it as u32))?.await?;
        let keys = store.get_all_keys(range_query, limit.map(|it| it as u32))?.await?;
        let result = keys.into_iter().zip(values).collect::<Vec<_>>();

        Ok(result
            .into_iter()
            .filter_map(|(key, value)| match (IdbId::deserialize(key), IdbTable::deserialize(value)) {
                (Err(e), _) => {
                    log::error!("Error deserializing record: {:?}", e);
                    None
                }
                (_, Err(e)) => {
                    log::error!("Error deserializing record: {:?}", e);
                    None
                }
                (Ok(key), Ok(record)) => Some((key, record))
            })
            .collect())
    }

    async fn delete_one(&self, id: &I) -> Resolve<T> {
        let table_name = T::get_table();

        let Some(record) = IdbRepository::find_one(self, id).await? else {
            return Err(RepositoryError::NotFound(table_name.to_string(), format!("{:?}", id)));
        };

        let db = self.get_db().await;
        let transaction = db.transaction(&[table_name], TransactionMode::ReadWrite)?;
        let store = transaction.object_store(table_name)?;
        let exact_id = IdbTable::id(&record);
        let key = IdbId::serialize(&exact_id)?;
        let query = Query::Key(key);
        store.delete(query)?.await?;
        Ok(record)
    }

    async fn update_one(&self, item: T) -> Resolve<T>
    where
        T: 'async_trait
    {
        let id = IdbTable::id(&item);
        let table_name = T::get_table();

        let Some(record) = IdbRepository::find_one(self, &id).await? else {
            return Err(RepositoryError::NotFound(
                table_name.to_owned(),
                format!("{:?}", IdbTable::id(&item))
            ));
        };

        let db = self.get_db().await;
        let table_name = T::get_table();
        let transaction = db.transaction(&[table_name], TransactionMode::ReadWrite)?;
        let store = transaction.object_store(table_name)?;
        let id = IdbTable::id(&record);
        let key = IdbId::serialize(&id)?;
        let value = IdbTable::serialize(&item)?;
        let updated_key = store.put(&value, Some(&key))?.await?;
        let Some(updated_value) = store.get(updated_key)?.await? else {
            return Err(RepositoryError::NotFound(table_name.to_owned(), format!("{:?}", id)));
        };

        Ok(IdbTable::deserialize(updated_value)?)
    }
}

// wasm32 implementation
#[cfg(target_arch = "wasm32")]
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
impl<X, T, I> Repository<T, I> for X
where
    X: IdbRepository<T, I> + Send + Sync,
    T: Send + Sync + IdbTable<I> + Clone,
    I: Send + Sync + IdbId + Clone
{
    async fn create(&self, data: T) -> Resolve<T>
    where
        T: 'async_trait
    {
        IdbRepository::create(self, data).await
    }

    async fn update_one(&self, data: T) -> Resolve<T>
    where
        T: 'async_trait
    {
        IdbRepository::update_one(self, data).await
    }

    async fn find_one(&self, record_id: &I) -> Resolve<Option<T>> {
        IdbRepository::find_one(self, record_id).await
    }

    async fn find_all(&self, r1: Option<&I>, r2: Option<&I>, count: Option<usize>) -> Resolve<Vec<T>> {
        IdbRepository::find_all(self, r1, r2, count).await
    }

    async fn delete_one(&self, record_id: &I) -> Resolve<T> {
        IdbRepository::delete_one(self, record_id).await
    }
}
