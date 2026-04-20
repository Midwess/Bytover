use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // p2p_session table
        manager
            .create_table(
                Table::create()
                    .table(P2PSession::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(P2PSession::SessionId).big_integer().not_null().primary_key())
                    .col(ColumnDef::new(P2PSession::DeviceId).big_integer().not_null())
                    .col(ColumnDef::new(P2PSession::UserId).big_integer().not_null())
                    .col(ColumnDef::new(P2PSession::Alias).text().not_null())
                    .col(ColumnDef::new(P2PSession::PasswordProtected).boolean().not_null().default(false))
                    .to_owned(),
            )
            .await?;

        // Index for device_id
        manager
            .create_index(
                Index::create()
                    .name("idx_p2p_session_device_id")
                    .table(P2PSession::Table)
                    .col(P2PSession::DeviceId)
                    .to_owned(),
            )
            .await?;

        // Index for alias (for find_session lookups)
        manager
            .create_index(
                Index::create()
                    .name("idx_p2p_session_alias")
                    .table(P2PSession::Table)
                    .col(P2PSession::Alias)
                    .to_owned(),
            )
            .await?;

        // Index for user_id
        manager
            .create_index(
                Index::create()
                    .name("idx_p2p_session_user_id")
                    .table(P2PSession::Table)
                    .col(P2PSession::UserId)
                    .to_owned(),
            )
            .await?;

        // Composite index for (user_id, device_id) for find_by_user_id_and_device_id queries
        manager
            .create_index(
                Index::create()
                    .name("idx_p2p_session_user_id_device_id")
                    .table(P2PSession::Table)
                    .col(P2PSession::UserId)
                    .col(P2PSession::DeviceId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop indexes first, then table
        manager
            .drop_index(Index::drop().name("idx_p2p_session_user_id_device_id").table(P2PSession::Table).to_owned())
            .await?;

        manager
            .drop_index(Index::drop().name("idx_p2p_session_user_id").table(P2PSession::Table).to_owned())
            .await?;

        manager
            .drop_index(Index::drop().name("idx_p2p_session_alias").table(P2PSession::Table).to_owned())
            .await?;

        manager
            .drop_index(Index::drop().name("idx_p2p_session_device_id").table(P2PSession::Table).to_owned())
            .await?;

        manager.drop_table(Table::drop().table(P2PSession::Table).to_owned()).await
    }
}

#[derive(DeriveIden)]
enum P2PSession {
    Table,
    SessionId,
    DeviceId,
    UserId,
    Alias,
    PasswordProtected,
}
