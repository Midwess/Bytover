pub use sea_orm_migration::prelude::*;

pub mod model;
mod m20220101_000001_create_table;
mod m20251227_000004_create_p2p_session_table;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_create_table::Migration),
            Box::new(m20251227_000004_create_p2p_session_table::Migration),
        ]
    }
}
