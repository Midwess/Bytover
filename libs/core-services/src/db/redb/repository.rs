use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};
use std::fmt::Debug;

use crate::db::redb::id::RedbId;
use crate::db::redb::table::RedbTable;
use crate::db::repository::abstraction::errors::{RepositoryError, Resolve};
use crate::db::repository::abstraction::repository::{Repository, SendSync};
use crate::utils::pool::reponse::PoolResponse;

#[async_trait::async_trait]
pub trait RedbRepository<T, I>: SendSync + Repository<T, I>
where
    T: SendSync + RedbTable<I> + Debug,
    I: SendSync + RedbId
{
    async fn get_db(&self) -> PoolResponse<Database>;

    fn ensure_table_exists(db: &Database, def: TableDefinition<&[u8], &[u8]>) -> Resolve<()> {
        let txn = db.begin_write()?;
        {
            txn.open_table(def)?;
        }

        txn.commit()?;

        Ok(())
    }

    async fn create(&self, item: T) -> Resolve<T>
    where
        T: 'async_trait
    {
        let table_name = T::get_table();
        let db = self.get_db().await;

        let tx = db.begin_write()?;
        {
            let table_def: TableDefinition<&[u8], &[u8]> = TableDefinition::new(table_name);
            let mut table = tx.open_table(table_def)?;
            let id = RedbTable::id(&item).lower_id();
            let key = Self::flatten_bytes_vec(&id);
            let value = RedbTable::serialize(&item)?;

            table.insert(key.as_slice(), value.as_slice())?;
        }

        tx.commit()?;
        Ok(item)
    }

    async fn update_one(&self, item: T) -> Resolve<T>
    where
        T: 'async_trait
    {
        if RedbRepository::find_one(self, &RedbTable::id(&item)).await?.is_some() {
            // Redb will override the existing item if the key already exists
            RedbRepository::create(self, item).await
        } else {
            Err(RepositoryError::NotFound(
                format!("{:?}: {:?}", T::get_table(), RedbTable::id(&item)),
                T::get_table().to_string()
            ))
        }
    }

    async fn find_one(&self, id: &I) -> Resolve<Option<T>> {
        let table_name = T::get_table();
        let db = self.get_db().await;
        Self::ensure_table_exists(&db, TableDefinition::new(table_name))?;

        let tx = db.begin_read()?;
        let table_def: TableDefinition<&[u8], &[u8]> = TableDefinition::new(table_name);
        let table = tx.open_table(table_def)?;
        let raw_from_key = id.lower_id();
        let from_key = Self::flatten_bytes_vec(&id.lower_id());
        let to_key = Self::flatten_bytes_vec(&id.upper_id());
        let value = match table.range(from_key.as_slice()..to_key.as_slice())?.next() {
            Some(value) => {
                let value = value?;
                Some(RedbTable::deserialize(value.1.value().to_vec()).unwrap())
            }
            None => {
                // Full table scan
                let none_bytes = bincode::serialize(&None::<T>).unwrap();
                let Some(item) = table.iter()?.find(|entry| {
                    let Ok(entry) = entry else {
                        return false;
                    };

                    let Ok(key) = bincode::deserialize::<Vec<Vec<u8>>>(entry.0.value().to_vec().as_slice()) else {
                        return false;
                    };

                    return key.iter().zip(&raw_from_key).all(|(record_key, expected_key)| {
                        if expected_key.eq(&none_bytes) {
                            return true;
                        }

                        record_key.eq(expected_key)
                    });
                }) else {
                    return Ok(None);
                };

                let value = item?;
                Some(RedbTable::deserialize(value.1.value().to_vec()).unwrap())
            }
        };

        Ok(value)
    }

    fn flatten_bytes_vec(parts: &Vec<Vec<u8>>) -> Vec<u8> {
        let bytes = bincode::serialize(parts).unwrap();
        bytes
    }

    async fn delete_one(&self, id: &I) -> Resolve<T> {
        let Some(item) = RedbRepository::find_one(self, id).await? else {
            return Err(RepositoryError::NotFound(
                format!("{:?}: {:?}", T::get_table(), id),
                T::get_table().to_string()
            ));
        };

        let table_name = T::get_table();
        let db = self.get_db().await;

        let id = RedbTable::<I>::id(&item).lower_id();
        let key = Self::flatten_bytes_vec(&id);

        let tx = db.begin_write()?;
        let table_def: TableDefinition<&[u8], &[u8]> = TableDefinition::new(table_name);

        {
            let mut table = tx.open_table(table_def)?;
            let removed = table.remove(&key[..])?;

            let bytes = match removed {
                Some(v) => v.value().to_vec(),
                None => {
                    return Err(RepositoryError::NotFound(
                        format!("{:?}: {:?}", T::get_table(), id),
                        T::get_table().to_string()
                    ))
                }
            };

            let deleted_item: T = RedbTable::deserialize(bytes)?;
            log::info!("Deleted item {:?}", deleted_item);
        }

        tx.commit()?;

        Ok(item)
    }

    async fn find_all(&self, from_id: Option<&I>, to_id: Option<&I>, count: Option<usize>) -> Resolve<Vec<T>> {
        let table_name = T::get_table();
        let db = self.get_db().await;
        Self::ensure_table_exists(&db, TableDefinition::new(table_name))?;
        let tx = db.begin_read()?;
        let table_def: TableDefinition<&[u8], &[u8]> = TableDefinition::new(table_name);
        let table = tx.open_table(table_def)?;
        let from_key = from_id.map(|id| Self::flatten_bytes_vec(&id.lower_id()));
        let to_key = to_id.map(|id| Self::flatten_bytes_vec(&id.lower_id()));

        let result = match (from_key, to_key) {
            (Some(from_key), Some(to_key)) => table.range(from_key.as_slice()..=to_key.as_slice())?,
            (Some(from_key), None) => table.range(from_key.as_slice()..)?,
            (None, Some(to_key)) => table.range(..=to_key.as_slice())?,
            (None, None) => table.iter()?
        };

        let mut items = Vec::new();
        for entry in result {
            let entry = entry?;
            if items.len() >= count.unwrap_or(usize::MAX) {
                break;
            }

            items.push(RedbTable::deserialize(entry.1.value().to_vec()).unwrap());
        }

        Ok(items)
    }
}

#[async_trait::async_trait]
impl<X, T, I> Repository<T, I> for X
where
    X: RedbRepository<T, I> + SendSync,
    T: SendSync + RedbTable<I> + Clone + Debug,
    I: SendSync + RedbId + Clone
{
    async fn create(&self, data: T) -> Resolve<T>
    where
        T: 'async_trait
    {
        RedbRepository::create(self, data).await
    }

    async fn update_one(&self, data: T) -> Resolve<T>
    where
        T: 'async_trait
    {
        RedbRepository::update_one(self, data).await
    }

    async fn find_one(&self, record_id: &I) -> Resolve<Option<T>> {
        RedbRepository::find_one(self, record_id).await
    }

    async fn find_all(&self, r1: Option<&I>, r2: Option<&I>, count: Option<usize>) -> Resolve<Vec<T>> {
        RedbRepository::find_all(self, r1, r2, count).await
    }

    async fn delete_one(&self, record_id: &I) -> Resolve<T> {
        RedbRepository::delete_one(self, record_id).await
    }
}
