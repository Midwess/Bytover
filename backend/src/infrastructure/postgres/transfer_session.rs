use crate::entities::transfer_session::TransferSession;
use crate::repositories::transfer_session::{TransferSessionId, TransferSessionRepository};
use core_services::db::repository::abstraction::errors::RepositoryError;
use core_services::db::repository::abstraction::repository::Repository;
use devlog_sdk::distributed_id::{gen_id, EPOCH_SINCE};
use sea_orm::entity::prelude::*;
use sea_orm::ActiveValue::Set;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseBackend, DatabaseConnection, EntityTrait, QueryFilter, Statement, Value as SeaValue};
use serde_json::{json, Value};
use chrono::{Utc, Months};

use migration::model::transfer_session as transfer_session_model;
use transfer_session_model::{
    ActiveModel as TransferSessionActiveModel,
    Column as TransferSessionColumn,
    Entity as TransferSessionEntity,
    Model as TransferSessionModel
};

pub struct TransferSessionPostgresRepository {
    pub db: DatabaseConnection
}

impl TryFrom<TransferSessionModel> for TransferSession {
    type Error = serde_json::Error;

    fn try_from(m: TransferSessionModel) -> Result<Self, Self::Error> {
        let progresses: Vec<Value> = match m.progress {
            Some(v) => serde_json::from_value(v)?,
            None => Vec::new()
        };
        let resources: Vec<Value> = match m.resources {
            Some(v) => serde_json::from_value(v)?,
            None => Vec::new()
        };
        let to_emails: Vec<String> = match m.to_emails {
            Some(v) => serde_json::from_value(v)?,
            None => Vec::new()
        };

        let v = json!({
            "order_id": m.order_id as u64,
            "owner_user_order_id": m.owner_user_order_id as u64,
            "alias": m.alias,
            "password": m.password,
            "to_emails": to_emails,
            "resources": resources,
            "progress": progresses,
        });

        serde_json::from_value(v)
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
            Some(model) => TransferSession::try_from(model).map(Some).map_err(|e| RepositoryError::DbError(e.to_string())),
            None => Ok(None)
        }
    }

    async fn delete_expired_or_canceled_sessions(&self) -> Result<(), RepositoryError> {
        let epoch_ms = EPOCH_SINCE as i64;
        
        // Compute cutoff timestamp in Rust (1 month ago in milliseconds)
        let one_month_ago = Utc::now() - Months::new(1);
        let one_month_ago_ms = one_month_ago.timestamp_millis();
        
        // Convert cutoff to order_id format by subtracting epoch and shifting
        let cutoff_order_id = (one_month_ago_ms - epoch_ms) << 23;        
        let statement = Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            r#"DELETE FROM transfer_session 
               WHERE order_id < $1
                  OR progress @> '[{"status": {"Failed": "Canceled"}}]'"#,
            vec![SeaValue::from(cutoff_order_id)]
        );

        self.db.execute(statement).await.map_err(|e| RepositoryError::DbError(e.to_string()))?;
        
        Ok(())
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
            resources: Set(Some(resources_val))
        };

        let _ = active.insert(&self.db).await.map_err(|e| RepositoryError::DbError(e.to_string()))?;

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

        let row = query.one(&self.db).await.map_err(|e| RepositoryError::DbError(e.to_string()))?;

        match row {
            Some(model) => TransferSession::try_from(model).map(Some).map_err(|e| RepositoryError::DbError(e.to_string())),
            None => Ok(None)
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

    async fn update_one(&self, data: TransferSession) -> Result<TransferSession, RepositoryError> {
        let session = TransferSessionEntity::find()
            .filter(TransferSessionColumn::OwnerUserOrderId.eq(data.user_order_id() as i64))
            .filter(TransferSessionColumn::OrderId.eq(data.order_id() as i64))
            .one(&self.db)
            .await
            .map_err(|e| RepositoryError::DbError(e.to_string()))?
            .ok_or_else(|| RepositoryError::DbError("Transfer session not found".to_string()))?;

        let alias = data.alias();
        let password = data.password();
        let to_emails_val = serde_json::to_value(data.to_emails()).map_err(|e| RepositoryError::DbError(e.to_string()))?;
        let resources_val = serde_json::to_value(data.resources()).map_err(|e| RepositoryError::DbError(e.to_string()))?;
        let progress_val = serde_json::to_value(data.progresses()).map_err(|e| RepositoryError::DbError(e.to_string()))?;

        let mut active: TransferSessionActiveModel = session.into();
        active.alias = Set(alias);
        active.password = Set(password.clone());
        active.to_emails = Set(Some(to_emails_val));
        active.resources = Set(Some(resources_val));
        active.progress = Set(Some(progress_val));

        active
            .update(&self.db)
            .await
            .map_err(|e| RepositoryError::DbError(e.to_string()))?;

        self.notify(&data).await?;

        Ok(data)
    }
}

impl TransferSessionPostgresRepository {
    async fn notify(&self, session: &TransferSession) -> Result<(), RepositoryError> {
        let channel = format!("transfer_session_{}_{}", session.user_order_id(), session.order_id());
        let payload = session.order_id().to_string();

        let statement = Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT pg_notify($1, $2)",
            vec![SeaValue::from(channel), SeaValue::from(payload)]
        );

        self
            .db
            .execute(statement)
            .await
            .map_err(|e| RepositoryError::DbError(e.to_string()))?;

        Ok(())
    }
}
