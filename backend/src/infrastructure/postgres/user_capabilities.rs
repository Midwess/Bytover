use crate::app_gateway::plan::{defaults_for, Plan};
use crate::entities::user_capabilities::UserCapabilities;
use crate::repositories::user_capabilities::{ClampedIncrementOutcome, IncrementOutcome, UserCapabilitiesRepository};
use core_services::db::repository::abstraction::errors::RepositoryError;
use migration::model::user_capabilities as model;
use sea_orm::ActiveValue::Set;
use sea_orm::{ActiveModelTrait, DatabaseBackend, DatabaseConnection, EntityTrait, FromQueryResult, Statement};

pub struct UserCapabilitiesPostgresRepository {
    pub db: DatabaseConnection,
}

impl UserCapabilitiesPostgresRepository {
    fn model_to_entity(m: model::Model) -> UserCapabilities {
        UserCapabilities::from_db(
            m.user_order_id as u64,
            Plan::from_i16(m.plan),
            m.password_encryption_allowed,
            m.max_files_per_transfer as u32,
            m.total_transfer_bytes_lifetime_cap as u64,
            m.total_transfer_bytes_used as u64,
            m.max_visible_shelves as u32,
            m.device_unique_key,
        )
    }

    async fn lookup_prior_free_bytes_for_device(
        &self,
        device_unique_key: &str,
        current_user_order_id: u64,
    ) -> Result<u64, RepositoryError> {
        let sql = r#"
            SELECT total_transfer_bytes_used
            FROM user_capabilities
            WHERE device_unique_key = $1
              AND plan = $2
              AND user_order_id <> $3
            ORDER BY total_transfer_bytes_used DESC
            LIMIT 1
        "#;

        let stmt = Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            sql,
            [
                device_unique_key.into(),
                (Plan::Free.as_i16() as i64).into(),
                (current_user_order_id as i64).into(),
            ],
        );

        let row = BytesUsedRow::find_by_statement(stmt)
            .one(&self.db)
            .await
            .map_err(|e| RepositoryError::DbError(e.to_string()))?;

        Ok(row.map(|r| r.total_transfer_bytes_used as u64).unwrap_or(0))
    }
}

#[derive(FromQueryResult)]
struct BytesUsedRow {
    total_transfer_bytes_used: i64,
}

#[derive(FromQueryResult)]
struct ClampedIncrementRow {
    prev_used: i64,
    new_used: i64,
    cap: i64,
}

#[async_trait::async_trait]
impl UserCapabilitiesRepository for UserCapabilitiesPostgresRepository {
    async fn find_or_create_default(
        &self,
        user_order_id: u64,
        device_unique_key: &str,
    ) -> Result<UserCapabilities, RepositoryError> {
        if let Some(m) = model::Entity::find_by_id(user_order_id as i64)
            .one(&self.db)
            .await
            .map_err(|e| RepositoryError::DbError(e.to_string()))?
        {
            return Ok(Self::model_to_entity(m));
        }

        let seeded_bytes_used = if device_unique_key.is_empty() {
            0
        } else {
            self.lookup_prior_free_bytes_for_device(device_unique_key, user_order_id).await?
        };

        let stored_device_key: Option<String> = if device_unique_key.is_empty() {
            None
        } else {
            Some(device_unique_key.to_owned())
        };

        let plan = Plan::Free;
        let defaults = defaults_for(plan);
        let now = chrono::Utc::now().into();
        let active = model::ActiveModel {
            user_order_id: Set(user_order_id as i64),
            plan: Set(plan.as_i16()),
            password_encryption_allowed: Set(defaults.password_encryption_allowed),
            max_files_per_transfer: Set(defaults.max_files_per_transfer as i32),
            total_transfer_bytes_lifetime_cap: Set(defaults.total_transfer_bytes_lifetime_cap as i64),
            total_transfer_bytes_used: Set(seeded_bytes_used as i64),
            max_visible_shelves: Set(defaults.max_visible_shelves as i32),
            device_unique_key: Set(stored_device_key),
            created_at: Set(now),
            updated_at: Set(now),
        };

        match active.insert(&self.db).await {
            Ok(m) => Ok(Self::model_to_entity(m)),
            Err(_) => {
                let m = model::Entity::find_by_id(user_order_id as i64)
                    .one(&self.db)
                    .await
                    .map_err(|e| RepositoryError::DbError(e.to_string()))?
                    .ok_or_else(|| RepositoryError::DbError("seed race: row missing after conflict".to_owned()))?;
                Ok(Self::model_to_entity(m))
            }
        }
    }

    async fn upgrade_to_paid(&self, user_order_id: u64) -> Result<UserCapabilities, RepositoryError> {
        self.find_or_create_default(user_order_id, "").await?;

        let existing = model::Entity::find_by_id(user_order_id as i64)
            .one(&self.db)
            .await
            .map_err(|e| RepositoryError::DbError(e.to_string()))?
            .ok_or_else(|| RepositoryError::DbError("upgrade_to_paid: row missing after seed".to_owned()))?;

        let paid_defaults = defaults_for(Plan::Paid);
        let now = chrono::Utc::now().into();
        let mut active: model::ActiveModel = existing.into();
        active.plan = Set(Plan::Paid.as_i16());
        active.password_encryption_allowed = Set(paid_defaults.password_encryption_allowed);
        active.max_files_per_transfer = Set(paid_defaults.max_files_per_transfer as i32);
        active.total_transfer_bytes_lifetime_cap = Set(paid_defaults.total_transfer_bytes_lifetime_cap as i64);
        active.max_visible_shelves = Set(paid_defaults.max_visible_shelves as i32);
        active.updated_at = Set(now);

        let updated = active
            .update(&self.db)
            .await
            .map_err(|e| RepositoryError::DbError(e.to_string()))?;

        Ok(Self::model_to_entity(updated))
    }

    async fn clamped_increment_bytes_used(
        &self,
        user_order_id: u64,
        delta: u64,
    ) -> Result<ClampedIncrementOutcome, RepositoryError> {
        let sql = r#"
            WITH prev AS (
                SELECT total_transfer_bytes_used, total_transfer_bytes_lifetime_cap
                FROM user_capabilities
                WHERE user_order_id = $2
            ),
            updated AS (
                UPDATE user_capabilities
                SET total_transfer_bytes_used = CASE
                    WHEN total_transfer_bytes_lifetime_cap = 0
                        THEN total_transfer_bytes_used + $1
                    ELSE LEAST(total_transfer_bytes_used + $1, total_transfer_bytes_lifetime_cap)
                END,
                updated_at = NOW()
                WHERE user_order_id = $2
                RETURNING total_transfer_bytes_used, total_transfer_bytes_lifetime_cap
            )
            SELECT
                prev.total_transfer_bytes_used AS prev_used,
                updated.total_transfer_bytes_used AS new_used,
                updated.total_transfer_bytes_lifetime_cap AS cap
            FROM prev, updated
        "#;

        let stmt = Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            sql,
            [(delta as i64).into(), (user_order_id as i64).into()],
        );

        let row = ClampedIncrementRow::find_by_statement(stmt)
            .one(&self.db)
            .await
            .map_err(|e| RepositoryError::DbError(e.to_string()))?
            .ok_or_else(|| RepositoryError::DbError("clamped_increment_bytes_used: row not found".to_owned()))?;

        let prev_used = row.prev_used as u64;
        let new_bytes_used = row.new_used as u64;
        let cap = row.cap as u64;
        let applied = new_bytes_used.saturating_sub(prev_used);
        let cap_exceeded = applied < delta;

        Ok(ClampedIncrementOutcome {
            applied,
            new_bytes_used,
            cap,
            cap_exceeded,
        })
    }

    async fn increment_bytes_used(&self, user_order_id: u64, delta: u64) -> Result<IncrementOutcome, RepositoryError> {
        let sql = r#"
            UPDATE user_capabilities
            SET total_transfer_bytes_used = total_transfer_bytes_used + $1,
                updated_at = NOW()
            WHERE user_order_id = $2
              AND (
                    total_transfer_bytes_lifetime_cap = 0
                    OR total_transfer_bytes_used + $1 <= total_transfer_bytes_lifetime_cap
                  )
            RETURNING total_transfer_bytes_used
        "#;

        let stmt = Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            sql,
            [(delta as i64).into(), (user_order_id as i64).into()],
        );

        if let Some(row) = BytesUsedRow::find_by_statement(stmt)
            .one(&self.db)
            .await
            .map_err(|e| RepositoryError::DbError(e.to_string()))?
        {
            return Ok(IncrementOutcome::Updated {
                new_bytes_used: row.total_transfer_bytes_used as u64,
            });
        }

        let current = model::Entity::find_by_id(user_order_id as i64)
            .one(&self.db)
            .await
            .map_err(|e| RepositoryError::DbError(e.to_string()))?
            .ok_or_else(|| RepositoryError::DbError("increment_bytes_used: row not found".to_owned()))?;

        Ok(IncrementOutcome::WouldExceedCap {
            cap: current.total_transfer_bytes_lifetime_cap as u64,
            used: current.total_transfer_bytes_used as u64,
            requested: delta,
        })
    }

}

