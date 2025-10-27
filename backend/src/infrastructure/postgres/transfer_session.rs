
use crate::entities::transfer_session::TransferSession;
use crate::repositories::transfer_session::{TransferSessionId, TransferSessionRepository};
use core_services::db::repository::abstraction::errors::RepositoryError;
use core_services::db::repository::abstraction::repository::Repository;
use sea_orm::entity::prelude::*;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, JsonValue};
use serde_json::{json, Value};

pub struct TransferSessionPostgresRepository {
    pub db: DatabaseConnection,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "transfer_session")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub alias: String,
    pub password: Option<String>,
    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub to_emails: Option<JsonValue>,
    pub order_id: i64,
    pub owner_user_order_id: i64,
    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub progress: Option<JsonValue>,
    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub resources: Option<JsonValue>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
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
        let row = Entity::find()
            .filter(Column::Alias.eq(alias))
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
    async fn create(&self, _data: TransferSession) -> Result<TransferSession, RepositoryError> {
        Err(RepositoryError::DbError("Not implemented for Postgres repository".to_string()))
    }

    async fn find_one(&self, id: &TransferSessionId) -> Result<Option<TransferSession>, RepositoryError> {
        let mut query = Entity::find();
        if let Some(user_order_id) = id.user_order_id {
            query = query.filter(Column::OwnerUserOrderId.eq(user_order_id as i64));
        }
        if let Some(order_id) = id.order_id {
            query = query.filter(Column::OrderId.eq(order_id as i64));
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