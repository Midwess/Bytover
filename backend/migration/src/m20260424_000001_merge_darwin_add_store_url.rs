use sea_orm_migration::prelude::*;

const BYTOVER_MACOS_APP_STORE_URL: &str = "https://apps.apple.com/app/bytover/id0000000000";

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("app_releases"))
                    .add_column(ColumnDef::new(AppRelease::StoreUrl).text().null())
                    .to_owned(),
            )
            .await?;

        manager
            .get_connection()
            .execute_unprepared("ALTER TABLE app_releases ALTER COLUMN download_url DROP NOT NULL")
            .await?;

        manager
            .get_connection()
            .execute_unprepared(
                "DELETE FROM app_releases WHERE platform = 'darwin' AND architecture = 'x86_64'",
            )
            .await?;

        manager
            .get_connection()
            .execute_unprepared(&format!(
                "UPDATE app_releases SET architecture = 'universal', download_url = NULL, store_url = '{BYTOVER_MACOS_APP_STORE_URL}' WHERE platform = 'darwin'"
            ))
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "INSERT INTO app_releases (version, platform, architecture, signature, download_url, release_notes, is_critical, created_at) \
                 SELECT version, 'darwin', 'x86_64', signature, \
                        'https://releases.bytover.com/darwin/x86_64/' || version, \
                        release_notes, is_critical, created_at \
                 FROM app_releases WHERE platform = 'darwin' AND architecture = 'universal'",
            )
            .await?;

        manager
            .get_connection()
            .execute_unprepared(
                "UPDATE app_releases SET architecture = 'aarch64', download_url = 'https://releases.bytover.com/darwin/aarch64/' || version, store_url = NULL WHERE platform = 'darwin' AND architecture = 'universal'",
            )
            .await?;

        manager
            .get_connection()
            .execute_unprepared("ALTER TABLE app_releases ALTER COLUMN download_url SET NOT NULL")
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("app_releases"))
                    .drop_column(AppRelease::StoreUrl)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum AppRelease {
    StoreUrl,
}
