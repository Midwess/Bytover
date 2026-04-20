use sea_orm::JsonValue;
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "transfer_session")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: i64,
    pub alias: String,
    pub password: Option<String>,
    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub to_emails: Option<JsonValue>,
    pub order_id: i64,
    pub owner_user_order_id: i64,
    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub progress: Option<JsonValue>,
    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub resources: Option<JsonValue>,
    pub status: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
