use crate::entities::device_alias::DeviceAlias;
use crate::repositories::device_alias::{DeviceAliasId, DeviceAliasRepository};
use core_services::db::repository::abstraction::errors::RepositoryError;
use core_services::db::repository::abstraction::repository::Repository;
use device_alias_model::{
    ActiveModel as DeviceAliasActiveModel, Column as DeviceAliasColumn, Entity as DeviceAliasEntity, Model as DeviceAliasModel,
};
use migration::model::device_alias as device_alias_model;
use sea_orm::ActiveValue::Set;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter};

pub struct DeviceAliasPostgresRepository {
    pub db: DatabaseConnection,
}

impl TryFrom<DeviceAliasModel> for DeviceAlias {
    type Error = RepositoryError;

    fn try_from(model: DeviceAliasModel) -> Result<Self, Self::Error> {
        Ok(DeviceAlias::from_db(model.alias, model.user_id as u64, model.device_id as u64))
    }
}

impl TryFrom<&DeviceAlias> for DeviceAliasActiveModel {
    type Error = RepositoryError;

    fn try_from(entity: &DeviceAlias) -> Result<Self, Self::Error> {
        Ok(DeviceAliasActiveModel {
            alias: Set(entity.alias().to_string()),
            user_id: Set(entity.user_id() as i64),
            device_id: Set(entity.device_id() as i64),
        })
    }
}

#[async_trait::async_trait]
impl Repository<DeviceAlias, DeviceAliasId> for DeviceAliasPostgresRepository {
    async fn create(&self, _data: DeviceAlias) -> Result<DeviceAlias, RepositoryError> {
        unimplemented!("Use create_alias instead")
    }

    async fn find_one(&self, _id: &DeviceAliasId) -> Result<Option<DeviceAlias>, RepositoryError> {
        unimplemented!("Use find_by_alias instead")
    }

    async fn find_all(
        &self,
        _from_id: Option<&DeviceAliasId>,
        _to_id: Option<&DeviceAliasId>,
        _count: Option<usize>,
    ) -> Result<Vec<DeviceAlias>, RepositoryError> {
        unimplemented!("Use find_by_user_and_device instead")
    }

    async fn delete_one(&self, _id: &DeviceAliasId) -> Result<DeviceAlias, RepositoryError> {
        unimplemented!("Not supported for DeviceAlias")
    }

    async fn update_one(&self, _data: DeviceAlias) -> Result<DeviceAlias, RepositoryError> {
        unimplemented!("Not supported for DeviceAlias")
    }
}

#[async_trait::async_trait]
impl DeviceAliasRepository for DeviceAliasPostgresRepository {
    async fn find_by_user_and_device(&self, user_id: u64, device_id: u64) -> Result<Vec<DeviceAlias>, RepositoryError> {
        let models = DeviceAliasEntity::find()
            .filter(DeviceAliasColumn::UserId.eq(user_id as i64))
            .filter(DeviceAliasColumn::DeviceId.eq(device_id as i64))
            .all(&self.db)
            .await
            .map_err(|e| RepositoryError::DbError(e.to_string()))?;

        models.into_iter().map(DeviceAlias::try_from).collect()
    }

    async fn find_by_alias(&self, alias: String) -> Result<Option<DeviceAlias>, RepositoryError> {
        let model = DeviceAliasEntity::find()
            .filter(DeviceAliasColumn::Alias.eq(alias))
            .one(&self.db)
            .await
            .map_err(|e| RepositoryError::DbError(e.to_string()))?;

        model.map(DeviceAlias::try_from).transpose()
    }

    async fn create_alias(&self, alias: DeviceAlias) -> Result<DeviceAlias, RepositoryError> {
        let active_model = DeviceAliasActiveModel::try_from(&alias)?;
        let result = active_model.insert(&self.db).await.map_err(|e| RepositoryError::DbError(e.to_string()))?;
        DeviceAlias::try_from(result)
    }

    async fn count_by_user_and_device(&self, user_id: u64, device_id: u64) -> Result<usize, RepositoryError> {
        let count = DeviceAliasEntity::find()
            .filter(DeviceAliasColumn::UserId.eq(user_id as i64))
            .filter(DeviceAliasColumn::DeviceId.eq(device_id as i64))
            .count(&self.db)
            .await
            .map_err(|e| RepositoryError::DbError(e.to_string()))?;

        Ok(count as usize)
    }

    async fn alias_exists(&self, alias: &str) -> Result<bool, RepositoryError> {
        let count = DeviceAliasEntity::find()
            .filter(DeviceAliasColumn::Alias.eq(alias))
            .count(&self.db)
            .await
            .map_err(|e| RepositoryError::DbError(e.to_string()))?;

        Ok(count > 0)
    }
}
