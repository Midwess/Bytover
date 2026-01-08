use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "device_alias")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub alias: String,
    pub user_id: i64,
    pub device_id: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
