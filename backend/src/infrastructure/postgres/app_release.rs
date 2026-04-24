use crate::entities::app_release;
use crate::repositories::app_release::{AppReleaseRepository, StoreReleaseUpsert};
use core_services::db::repository::abstraction::errors::RepositoryError;
use sea_orm::{DatabaseBackend, DatabaseConnection, EntityTrait, Statement};

pub struct AppReleasePostgresRepository {
    pub db: DatabaseConnection,
}

const UPSERT_STORE_RELEASE_SQL: &str = r#"
    INSERT INTO app_releases (
        version,
        platform,
        architecture,
        signature,
        download_url,
        release_notes,
        is_critical,
        store_url,
        created_at
    )
    VALUES ($1, $2, 'universal', '', NULL, $3, false, $4, NOW())
    ON CONFLICT (platform, version)
    DO UPDATE SET
        store_url = EXCLUDED.store_url,
        release_notes = EXCLUDED.release_notes,
        download_url = NULL,
        architecture = 'universal'
"#;

#[async_trait::async_trait]
impl AppReleaseRepository for AppReleasePostgresRepository {
    async fn upsert_store_release(&self, row: StoreReleaseUpsert) -> Result<(), RepositoryError> {
        let stmt = Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            UPSERT_STORE_RELEASE_SQL,
            [
                row.version.into(),
                row.platform.into(),
                row.release_notes.into(),
                row.store_url.into(),
            ],
        );

        <sea_orm::DatabaseConnection as sea_orm::ConnectionTrait>::execute(&self.db, stmt)
            .await
            .map_err(|e| RepositoryError::DbError(e.to_string()))?;

        Ok(())
    }

    async fn latest_for_platform(&self, platform: &str) -> Result<Option<app_release::Model>, RepositoryError> {
        let rows = app_release::Entity::find()
            .all(&self.db)
            .await
            .map_err(|e| RepositoryError::DbError(e.to_string()))?;

        let latest = rows
            .into_iter()
            .filter(|r| r.platform == platform)
            .filter_map(|r| semver::Version::parse(&r.version).ok().map(|v| (r, v)))
            .max_by(|a, b| a.1.cmp(&b.1))
            .map(|(row, _)| row);

        Ok(latest)
    }
}
