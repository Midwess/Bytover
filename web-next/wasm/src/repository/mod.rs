use async_trait::async_trait;
use futures::executor::block_on;
use idb::{Database, Factory};
use core_services::utils::never_send::NeverSend;
use core_services::utils::pool::allocator::PoolResourceProvider;

pub mod path_resolver;
pub mod id;
pub mod auth_session;
pub mod local_resource;
pub mod transfer_session;

pub mod user;
mod errors;

pub struct IdbPoolProvider {
    pub name: String
}

#[async_trait(?Send)]
impl PoolResourceProvider<NeverSend<Database>> for IdbPoolProvider {
    async fn new(&self) -> NeverSend<Database>
    where
        Self: 'async_trait
    {
        let factory = NeverSend(Factory::new().unwrap());
        let open_request = factory.open("db", Some(1)).unwrap();

        NeverSend(open_request.await.unwrap())
    }
}
