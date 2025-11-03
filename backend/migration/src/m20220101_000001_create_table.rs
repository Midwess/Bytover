use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // transfer_session table
        manager
            .create_table(
                Table::create()
                    .table(TransferSession::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(TransferSession::Id)
                            .big_integer()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(TransferSession::Alias)
                            .text()
                            .not_null(),
                    )
                    .col(ColumnDef::new(TransferSession::Password).text().null())
                    .col(
                        ColumnDef::new(TransferSession::ToEmails)
                            .json_binary()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(TransferSession::OrderId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(TransferSession::OwnerUserOrderId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(TransferSession::Progress)
                            .json_binary()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(TransferSession::Resources)
                            .json_binary()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        // Unique constraint on (owner_user_order_id, order_id)
        manager
            .create_index(
                Index::create()
                    .name("uq_transfer_session_owner_user_order_id_order_id")
                    .table(TransferSession::Table)
                    .col(TransferSession::OwnerUserOrderId)
                    .col(TransferSession::OrderId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // Non-unique index for alias
        manager
            .create_index(
                Index::create()
                    .name("idx_transfer_session_alias")
                    .table(TransferSession::Table)
                    .col(TransferSession::Alias)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop indexes first, then table
        manager
            .drop_index(
                Index::drop()
                    .name("idx_transfer_session_alias")
                    .table(TransferSession::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("uq_transfer_session_owner_user_order_id_order_id")
                    .table(TransferSession::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(Table::drop().table(TransferSession::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum TransferSession {
    Table,
    Id,
    Alias,
    Password,
    ToEmails,
    OrderId,
    OwnerUserOrderId,
    Progress,
    Resources,
}
