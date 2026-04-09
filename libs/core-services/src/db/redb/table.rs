use crate::db::redb::id::RedbId;
use crate::db::repository::abstraction::errors::RepositoryError;
use crate::db::repository::abstraction::table::Table;

pub trait RedbTable<T>: Table<T> + serde::Serialize + for<'de> serde::Deserialize<'de>
where
    T: RedbId
{
    fn id(&self) -> T {
        Table::id(self)
    }

    fn serialize(&self) -> Result<Vec<u8>, RepositoryError> {
        Ok(bincode::serialize(self)?)
    }

    fn deserialize(value: Vec<u8>) -> Result<Self, RepositoryError> {
        Ok(bincode::deserialize(&value)?)
    }
}
