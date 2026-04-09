use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Insert darwin-aarch64
        manager
            .exec_stmt(
                Query::insert()
                    .into_table(Alias::new("app_releases"))
                    .columns([
                        AppRelease::Version,
                        AppRelease::Platform,
                        AppRelease::Architecture,
                        AppRelease::Signature,
                        AppRelease::DownloadUrl,
                        AppRelease::ReleaseNotes,
                        AppRelease::IsCritical,
                    ])
                    .values_panic([
                        "1.0.0".into(),
                        "darwin".into(),
                        "aarch64".into(),
                        "".into(),
                        "https://releases.bytover.com/darwin/aarch64/1.0.0".into(),
                        "Initial release".into(),
                        false.into(),
                    ])
                    .to_owned(),
            )
            .await?;

        // Insert darwin-x86_64
        manager
            .exec_stmt(
                Query::insert()
                    .into_table(Alias::new("app_releases"))
                    .columns([
                        AppRelease::Version,
                        AppRelease::Platform,
                        AppRelease::Architecture,
                        AppRelease::Signature,
                        AppRelease::DownloadUrl,
                        AppRelease::ReleaseNotes,
                        AppRelease::IsCritical,
                    ])
                    .values_panic([
                        "1.0.0".into(),
                        "darwin".into(),
                        "x86_64".into(),
                        "".into(),
                        "https://releases.bytover.com/darwin/x86_64/1.0.0".into(),
                        "Initial release".into(),
                        false.into(),
                    ])
                    .to_owned(),
            )
            .await?;

        // Insert linux-x86_64
        manager
            .exec_stmt(
                Query::insert()
                    .into_table(Alias::new("app_releases"))
                    .columns([
                        AppRelease::Version,
                        AppRelease::Platform,
                        AppRelease::Architecture,
                        AppRelease::Signature,
                        AppRelease::DownloadUrl,
                        AppRelease::ReleaseNotes,
                        AppRelease::IsCritical,
                    ])
                    .values_panic([
                        "1.0.0".into(),
                        "linux".into(),
                        "x86_64".into(),
                        "".into(),
                        "https://releases.bytover.com/linux/x86_64/1.0.0".into(),
                        "Initial release".into(),
                        false.into(),
                    ])
                    .to_owned(),
            )
            .await?;

        // Insert windows-x86_64
        manager
            .exec_stmt(
                Query::insert()
                    .into_table(Alias::new("app_releases"))
                    .columns([
                        AppRelease::Version,
                        AppRelease::Platform,
                        AppRelease::Architecture,
                        AppRelease::Signature,
                        AppRelease::DownloadUrl,
                        AppRelease::ReleaseNotes,
                        AppRelease::IsCritical,
                    ])
                    .values_panic([
                        "1.0.0".into(),
                        "windows".into(),
                        "x86_64".into(),
                        "".into(),
                        "https://releases.bytover.com/windows/x86_64/1.0.0".into(),
                        "Initial release".into(),
                        false.into(),
                    ])
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .exec_stmt(
                Query::delete()
                    .from_table(Alias::new("app_releases"))
                    .and_where(Expr::col(AppRelease::Version).eq("1.0.0"))
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum AppRelease {
    Version,
    Platform,
    Architecture,
    Signature,
    DownloadUrl,
    ReleaseNotes,
    IsCritical,
}
