use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "user_capabilities")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub user_order_id: i64,
    pub plan: i16,
    pub password_encryption_allowed: bool,
    pub max_files_per_transfer: i32,
    pub total_transfer_bytes_lifetime_cap: i64,
    pub total_transfer_bytes_used: i64,
    pub max_visible_shelves: i32,
    pub device_unique_key: Option<String>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
