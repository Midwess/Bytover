pub use sea_orm_migration::prelude::*;

mod m20220101_000001_create_table;
mod m20250108_000006_create_device_alias_table;
mod m20250309_000001_create_app_releases_table;
mod m20250309_000002_seed_app_releases;
mod m20251227_000004_create_p2p_session_table;
mod m20251229_000005_update_p2p_session_description;
mod m20260328_000001_add_signalling_key_to_p2p_session;
mod m20260408_000001_add_signalling_route_to_p2p_session;
mod m20260423_000001_create_user_capabilities_table;
pub mod model;

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
            Box::new(m20250309_000002_seed_app_releases::Migration),
            Box::new(m20260328_000001_add_signalling_key_to_p2p_session::Migration),
            Box::new(m20260408_000001_add_signalling_route_to_p2p_session::Migration),
            Box::new(m20260423_000001_create_user_capabilities_table::Migration),
        ]
    }
}
