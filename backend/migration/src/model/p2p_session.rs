use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "p2p_session")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub session_id: i64,
    pub device_id: i64,
    pub user_id: i64,
    pub alias: String,
    pub description: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
