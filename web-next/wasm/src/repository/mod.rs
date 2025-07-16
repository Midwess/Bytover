use async_trait::async_trait;
use core_services::db::repository::abstraction::table::Table;
use futures::executor::block_on;
use idb::{Database, DatabaseEvent, Factory, KeyPath, ObjectStoreParams};
use core_services::utils::never_send::NeverSend;
use core_services::utils::pool::allocator::PoolResourceProvider;
use shared::app::file_system::file::LocalResource;
use shared::app::repository::auth_session::AuthSessionId;
use shared::app::repository::local_resource::LocalResourceId;
use shared::app::repository::transfer_session::TransferSessionId;
use shared::app::repository::user::UserId;
use shared::app::transfer::session::TransferSession;
use shared::entities::session::Session;
use shared::entities::user::User;
use crate::repository::id::IdbIdWrapper;

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
        let mut open_request = factory.open("db", Some(1)).unwrap();
        open_request.on_upgrade_needed(|event| {
            let database = event.database().unwrap();
            let mut store_params = ObjectStoreParams::new();
            store_params.auto_increment(true);
            store_params.key_path(Some(KeyPath::new_single("id")));

            database.create_object_store(
                "authSession",
                store_params.clone()
            ).unwrap();
            database.create_object_store(
                "user",
                store_params.clone()
            ).unwrap();
            database.create_object_store(
                "localResource",
                store_params.clone()
            ).unwrap();
            database.create_object_store(
                "transferSession",
                store_params.clone()
            ).unwrap();
        });

        NeverSend(open_request.await.unwrap())
    }
}
