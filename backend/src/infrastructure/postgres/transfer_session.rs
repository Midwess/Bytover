
use crate::entities::transfer_session::TransferSession;
use crate::repositories::transfer_session::{TransferSessionId, TransferSessionRepository};
use core_services::db::repository::abstraction::errors::RepositoryError;
use core_services::db::repository::abstraction::repository::Repository;
use sea_orm::entity::prelude::*;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use sea_orm::ActiveValue::Set;
use devlog_sdk::distributed_id::gen_id;
use serde_json::{json, Value};

use migration::model::transfer_session as transfer_session_model;
use transfer_session_model::{ActiveModel as TransferSessionActiveModel, Column as TransferSessionColumn, Entity as TransferSessionEntity, Model as TransferSessionModel};

pub struct TransferSessionPostgresRepository {
    pub db: DatabaseConnection,
}

trait TransferSessionModelExt {
    fn into_domain(self) -> Result<TransferSession, RepositoryError>;
}

impl TransferSessionModelExt for TransferSessionModel {
    fn into_domain(self) -> Result<TransferSession, RepositoryError> {
        let progresses: Vec<Value> = match self.progress {
            Some(v) => serde_json::from_value(v)?,
            None => Vec::new(),
        };
        let resources : Vec<Value>= match self.resources {
            Some(v) => serde_json::from_value(v)?,
            None => Vec::new(),
        };
        let to_emails: Vec<String> = match self.to_emails {
            Some(v) => serde_json::from_value(v)?,
            None => Vec::new(),
        };

        let value = json!({
            "order_id": self.order_id as u64,
            "owner_user_order_id": self.owner_user_order_id as u64,
            "alias": self.alias,
            "password": self.password,
            "to_emails": to_emails,
            "resources": resources,
            "progress": progresses,
        });

        let session: TransferSession = serde_json::from_value(value)?;
        Ok(session)
    }
}

#[async_trait::async_trait]
impl TransferSessionRepository for TransferSessionPostgresRepository {
    async fn find_session_by_alias(&self, alias: String) -> Result<Option<TransferSession>, RepositoryError> {
        let row = TransferSessionEntity::find()
            .filter(TransferSessionColumn::Alias.eq(alias))
            .one(&self.db)
            .await
            .map_err(|e| RepositoryError::DbError(e.to_string()))?;

        match row {
            Some(model) => model.into_domain().map(Some),
            None => Ok(None),
        }
    }
}

#[async_trait::async_trait]
impl Repository<TransferSession, TransferSessionId> for TransferSessionPostgresRepository {
    async fn create(&self, data: TransferSession) -> Result<TransferSession, RepositoryError> {
        let row_id = gen_id().await as i64;
        let password = data.password();
        let to_emails_val = serde_json::to_value(data.to_emails()).map_err(|e| RepositoryError::DbError(e.to_string()))?;
        let resources_val = serde_json::to_value(data.resources()).map_err(|e| RepositoryError::DbError(e.to_string()))?;
        let progress_val = serde_json::to_value(data.progresses()).map_err(|e| RepositoryError::DbError(e.to_string()))?;

        let active = TransferSessionActiveModel {
            id: Set(row_id),
            alias: Set(data.alias()),
            password: Set(password),
            to_emails: Set(Some(to_emails_val)),
            order_id: Set(data.order_id() as i64),
            owner_user_order_id: Set(data.user_order_id() as i64),
            progress: Set(Some(progress_val)),
            resources: Set(Some(resources_val)),
        };

        let _ = active
            .insert(&self.db)
            .await
            .map_err(|e| RepositoryError::DbError(e.to_string()))?;

        Ok(data)
    }

    async fn find_one(&self, id: &TransferSessionId) -> Result<Option<TransferSession>, RepositoryError> {
        let mut query = TransferSessionEntity::find();
        if let Some(user_order_id) = id.user_order_id {
            query = query.filter(TransferSessionColumn::OwnerUserOrderId.eq(user_order_id as i64));
        }
        if let Some(order_id) = id.order_id {
            query = query.filter(TransferSessionColumn::OrderId.eq(order_id as i64));
        }

        let row = query
            .one(&self.db)
            .await
            .map_err(|e| RepositoryError::DbError(e.to_string()))?;

        match row {
            Some(model) => model.into_domain().map(Some),
            None => Ok(None),
        }
    }

    async fn find_all(
        &self,
        _from_id: Option<&TransferSessionId>,
        _to_id: Option<&TransferSessionId>,
        _count: Option<usize>
    ) -> Result<Vec<TransferSession>, RepositoryError> {
        Err(RepositoryError::DbError("Not implemented for Postgres repository".to_string()))
    }

    async fn delete_one(&self, _id: &TransferSessionId) -> Result<TransferSession, RepositoryError> {
        Err(RepositoryError::DbError("Not implemented for Postgres repository".to_string()))
    }

    async fn update_one(&self, _data: TransferSession) -> Result<TransferSession, RepositoryError> {
        Err(RepositoryError::DbError("Not implemented for Postgres repository".to_string()))
    }
}