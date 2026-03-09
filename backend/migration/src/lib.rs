pub use sea_orm_migration::prelude::*;

pub mod model;
mod m20220101_000001_create_table;
mod m20251227_000004_create_p2p_session_table;
mod m20251229_000005_update_p2p_session_description;
mod m20250108_000006_create_device_alias_table;
mod m20250309_000001_create_app_releases_table;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_create_table::Migration),
            Box::new(m20251227_000004_create_p2p_session_table::Migration),
            Box::new(m20251229_000005_update_p2p_session_description::Migration),
            Box::new(m20250108_000006_create_device_alias_table::Migration),
            Box::new(m20250309_000001_create_app_releases_table::Migration),
        ]
    }
}
