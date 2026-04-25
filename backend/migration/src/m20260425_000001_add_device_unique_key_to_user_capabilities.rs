use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(UserCapabilities::Table)
                    .add_column(ColumnDef::new(UserCapabilities::DeviceUniqueKey).text().null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("user_capabilities_device_unique_key_plan_idx")
                    .table(UserCapabilities::Table)
                    .col(UserCapabilities::DeviceUniqueKey)
                    .col(UserCapabilities::Plan)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("user_capabilities_device_unique_key_plan_idx")
                    .table(UserCapabilities::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(UserCapabilities::Table)
                    .drop_column(UserCapabilities::DeviceUniqueKey)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum UserCapabilities {
    Table,
    Plan,
    DeviceUniqueKey,
}
