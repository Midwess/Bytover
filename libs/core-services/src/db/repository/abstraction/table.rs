use super::id::DbId;

pub trait Table<T>
where
    T: DbId
{
    fn get_table() -> &'static str;

    fn id(&self) -> T;
}
