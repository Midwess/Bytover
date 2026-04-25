use sea_orm_migration::prelude::*;

const OLD_CAP_BYTES: i64 = 8 * 1024 * 1024 * 1024;
const NEW_CAP_BYTES: i64 = 20 * 1024 * 1024 * 1024;
const PLAN_FREE: i16 = 1;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared(&format!(
            "UPDATE user_capabilities \
             SET total_transfer_bytes_lifetime_cap = {NEW_CAP_BYTES}, \
                 updated_at = NOW() \
             WHERE plan = {PLAN_FREE} \
               AND total_transfer_bytes_lifetime_cap = {OLD_CAP_BYTES}"
        ))
        .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared(&format!(
            "UPDATE user_capabilities \
             SET total_transfer_bytes_lifetime_cap = {OLD_CAP_BYTES}, \
                 updated_at = NOW() \
             WHERE plan = {PLAN_FREE} \
               AND total_transfer_bytes_lifetime_cap = {NEW_CAP_BYTES}"
        ))
        .await?;
        Ok(())
    }
}
