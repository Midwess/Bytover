use super::errors::Resolve;
use super::id::DbId;
use super::table::Table;

#[cfg(target_family = "wasm")]
pub trait SendSync: Sync {}
#[cfg(not(target_family = "wasm"))]
pub trait SendSync: Sync + Send {}

#[cfg(target_family = "wasm")]
impl<T> SendSync for T where T: Sync {}

#[cfg(not(target_family = "wasm"))]
impl<T> SendSync for T where T: Sync + Send {}

#[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
#[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
pub trait Repository<T, I>: Send + Sync
where
    T: SendSync + Table<I>,
    I: SendSync + DbId
{
    async fn create(&self, data: T) -> Resolve<T>
    where
        T: 'async_trait;

    async fn find_one(&self, id: &I) -> Resolve<Option<T>>;

    async fn find_all(&self, from_id: Option<&I>, to_id: Option<&I>, count: Option<usize>) -> Resolve<Vec<T>>;

    async fn delete_one(&self, id: &I) -> Resolve<T>;

    async fn update_one(&self, data: T) -> Resolve<T>
    where
        T: 'async_trait;
}
