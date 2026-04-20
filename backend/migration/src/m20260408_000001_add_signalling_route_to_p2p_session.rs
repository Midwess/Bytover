use sea_orm_migration::prelude::*;

const DEFAULT_SIGNALLING_ROUTE: &str = "rpc-signalling-local";

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(P2PSession::Table)
                    .add_column(ColumnDef::new(P2PSession::SignallingRoute).text().not_null().default(DEFAULT_SIGNALLING_ROUTE))
                    .to_owned(),
            )
            .await?;

        manager
            .get_connection()
            .execute_unprepared(&format!(
                "UPDATE p2p_session SET signalling_route = '{DEFAULT_SIGNALLING_ROUTE}' WHERE signalling_route IS NULL OR signalling_route = ''"
            ))
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(Table::alter().table(P2PSession::Table).drop_column(P2PSession::SignallingRoute).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum P2PSession {
    Table,
    SignallingRoute,
}
