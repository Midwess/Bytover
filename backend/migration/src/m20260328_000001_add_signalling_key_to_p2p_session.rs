use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(P2PSession::Table)
                    .add_column(ColumnDef::new(P2PSession::SignallingKey).text().not_null().default(""))
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(Table::alter().table(P2PSession::Table).drop_column(P2PSession::SignallingKey).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum P2PSession {
    Table,
    SignallingKey,
}
