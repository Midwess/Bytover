use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(UserCapabilities::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(UserCapabilities::UserOrderId).big_integer().not_null().primary_key())
                    .col(ColumnDef::new(UserCapabilities::Plan).small_integer().not_null())
                    .col(ColumnDef::new(UserCapabilities::PasswordEncryptionAllowed).boolean().not_null())
                    .col(ColumnDef::new(UserCapabilities::MaxFilesPerTransfer).integer().not_null())
                    .col(ColumnDef::new(UserCapabilities::TotalTransferBytesLifetimeCap).big_integer().not_null())
                    .col(
                        ColumnDef::new(UserCapabilities::TotalTransferBytesUsed)
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .col(ColumnDef::new(UserCapabilities::MaxVisibleShelves).integer().not_null())
                    .col(
                        ColumnDef::new(UserCapabilities::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(UserCapabilities::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(UserCapabilities::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum UserCapabilities {
    Table,
    UserOrderId,
    Plan,
    PasswordEncryptionAllowed,
    MaxFilesPerTransfer,
    TotalTransferBytesLifetimeCap,
    TotalTransferBytesUsed,
    MaxVisibleShelves,
    CreatedAt,
    UpdatedAt,
}
