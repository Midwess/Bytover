use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_index(
                Index::create()
                    .name("uq_app_releases_platform_version")
                    .table(Alias::new("app_releases"))
                    .col(Alias::new("platform"))
                    .col(Alias::new("version"))
                    .unique()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("uq_app_releases_platform_version")
                    .table(Alias::new("app_releases"))
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}
