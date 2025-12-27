use crate::entities::p2p_session::P2PSession;
use crate::repositories::p2p_session::{P2PSessionId, P2PSessionRepository};
use core_services::db::repository::abstraction::errors::RepositoryError;
use core_services::db::repository::abstraction::repository::Repository;
use migration::model::p2p_session as p2p_session_model;
use p2p_session_model::{
    ActiveModel as P2PSessionActiveModel,
    Column as P2PSessionColumn,
    Entity as P2PSessionEntity,
    Model as P2PSessionModel
};
use sea_orm::ActiveValue::Set;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

pub struct P2PSessionPostgresRepository {
    pub db: DatabaseConnection
}

impl TryFrom<P2PSessionModel> for P2PSession {
    type Error = RepositoryError;

    fn try_from(model: P2PSessionModel) -> Result<Self, Self::Error> {
        Ok(P2PSession::from_db(
            model.session_id as u64,
            model.device_id as u64,
            model.user_id as u64,
            model.alias,
            model.password_protected,
        ))
    }
}

impl TryFrom<&P2PSession> for P2PSessionActiveModel {
    type Error = RepositoryError;

    fn try_from(entity: &P2PSession) -> Result<Self, Self::Error> {
        Ok(P2PSessionActiveModel {
            session_id: Set(entity.session_id() as i64),
            device_id: Set(entity.device_id() as i64),
            user_id: Set(entity.user_id() as i64),
            alias: Set(entity.alias().to_string()),
            password_protected: Set(entity.password_protected()),
        })
    }
}

#[async_trait::async_trait]
impl Repository<P2PSession, P2PSessionId> for P2PSessionPostgresRepository {
    async fn create(&self, _data: P2PSession) -> Result<P2PSession, RepositoryError> {
        unimplemented!("Use create_session instead")
    }

    async fn find_one(&self, _id: &P2PSessionId) -> Result<Option<P2PSession>, RepositoryError> {
        unimplemented!("Use find_by_alias or find_by_user_id_and_device_id instead")
    }

    async fn find_all(
        &self,
        _from_id: Option<&P2PSessionId>,
        _to_id: Option<&P2PSessionId>,
        _count: Option<usize>
    ) -> Result<Vec<P2PSession>, RepositoryError> {
        unimplemented!("Not supported for P2PSession")
    }

    async fn delete_one(&self, _id: &P2PSessionId) -> Result<P2PSession, RepositoryError> {
        unimplemented!("Not supported for P2PSession")
    }

    async fn update_one(&self, _data: P2PSession) -> Result<P2PSession, RepositoryError> {
        unimplemented!("Use update_session instead")
    }
}

#[async_trait::async_trait]
impl P2PSessionRepository for P2PSessionPostgresRepository {
    async fn find_by_user_id_and_device_id(
        &self,
        user_id: u64,
        device_id: u64,
    ) -> Result<Option<P2PSession>, RepositoryError> {
        let model = P2PSessionEntity::find()
            .filter(P2PSessionColumn::UserId.eq(user_id as i64))
            .filter(P2PSessionColumn::DeviceId.eq(device_id as i64))
            .one(&self.db)
            .await
            .map_err(|e| RepositoryError::DbError(e.to_string()))?;

        model.map(P2PSession::try_from).transpose()
    }

    async fn find_by_alias(&self, alias: String) -> Result<Option<P2PSession>, RepositoryError> {
        let model = P2PSessionEntity::find()
            .filter(P2PSessionColumn::Alias.eq(alias))
            .one(&self.db)
            .await
            .map_err(|e| RepositoryError::DbError(e.to_string()))?;

        model.map(P2PSession::try_from).transpose()
    }

    async fn create_session(&self, session: P2PSession) -> Result<P2PSession, RepositoryError> {
        let active_model = P2PSessionActiveModel::try_from(&session)?;
        let result = active_model
            .insert(&self.db)
            .await
            .map_err(|e| RepositoryError::DbError(e.to_string()))?;
        P2PSession::try_from(result)
    }

    async fn update_session(&self, session: P2PSession) -> Result<P2PSession, RepositoryError> {
        let active_model = P2PSessionActiveModel::try_from(&session)?;
        let result = active_model
            .update(&self.db)
            .await
            .map_err(|e| RepositoryError::DbError(e.to_string()))?;
        P2PSession::try_from(result)
    }
}
