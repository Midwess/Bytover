use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Alias::new("app_releases"))
                    .if_not_exists()
                    .col(
                        ColumnDef::new(AppRelease::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(AppRelease::Version)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AppRelease::Platform)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AppRelease::Architecture)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AppRelease::Signature)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AppRelease::DownloadUrl)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AppRelease::ReleaseNotes)
                            .text()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(AppRelease::IsCritical)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(AppRelease::CreatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_app_releases_version")
                    .table(Alias::new("app_releases"))
                    .col(AppRelease::Version)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_app_releases_platform_arch")
                    .table(Alias::new("app_releases"))
                    .col(AppRelease::Platform)
                    .col(AppRelease::Architecture)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("idx_app_releases_platform_arch")
                    .table(Alias::new("app_releases"))
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_app_releases_version")
                    .table(Alias::new("app_releases"))
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(Table::drop().table(Alias::new("app_releases")).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum AppRelease {
    Table,
    Id,
    Version,
    Platform,
    Architecture,
    Signature,
    DownloadUrl,
    ReleaseNotes,
    IsCritical,
    CreatedAt,
}
