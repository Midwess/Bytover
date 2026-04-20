use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(DeviceAlias::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(DeviceAlias::Alias).text().not_null().primary_key())
                    .col(ColumnDef::new(DeviceAlias::UserId).big_integer().not_null())
                    .col(ColumnDef::new(DeviceAlias::DeviceId).big_integer().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_device_alias_user_device")
                    .table(DeviceAlias::Table)
                    .col(DeviceAlias::UserId)
                    .col(DeviceAlias::DeviceId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(Index::drop().name("idx_device_alias_user_device").table(DeviceAlias::Table).to_owned())
            .await?;

        manager.drop_table(Table::drop().table(DeviceAlias::Table).to_owned()).await
    }
}

#[derive(DeriveIden)]
enum DeviceAlias {
    Table,
    Alias,
    UserId,
    DeviceId,
}
