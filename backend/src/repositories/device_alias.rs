use crate::entities::device_alias::DeviceAlias;
use core_services::db::repository::abstraction::errors::RepositoryError;
use core_services::db::repository::abstraction::id::DbId;
use core_services::db::repository::abstraction::repository::Repository;
use core_services::db::repository::abstraction::table::Table;

#[derive(Clone, Default)]
pub struct DeviceAliasId {
    pub alias: Option<String>,
}

impl Table<DeviceAliasId> for DeviceAlias {
    fn get_table() -> &'static str {
        "device_alias"
    }

    fn id(&self) -> DeviceAliasId {
        DeviceAliasId {
            alias: Some(self.alias().to_string()),
        }
    }
}

impl DbId for DeviceAliasId {
    type Table = DeviceAlias;
}

#[async_trait::async_trait]
pub trait DeviceAliasRepository: Repository<DeviceAlias, DeviceAliasId> {
    async fn find_by_user_and_device(
        &self,
        user_id: u64,
        device_id: u64,
    ) -> Result<Vec<DeviceAlias>, RepositoryError>;

    async fn find_by_alias(&self, alias: String) -> Result<Option<DeviceAlias>, RepositoryError>;

    async fn create_alias(&self, alias: DeviceAlias) -> Result<DeviceAlias, RepositoryError>;

    async fn count_by_user_and_device(
        &self,
        user_id: u64,
        device_id: u64,
    ) -> Result<usize, RepositoryError>;

    async fn alias_exists(&self, alias: &str) -> Result<bool, RepositoryError>;
}
