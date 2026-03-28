use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "app_releases")]
pub struct Model {
    #[sea_orm(column_name = "id", primary_key)]
    pub id: i64,
    #[sea_orm(column_name = "version")]
    pub version: String,
    #[sea_orm(column_name = "platform")]
    pub platform: String,
    #[sea_orm(column_name = "architecture")]
    pub architecture: String,
    #[sea_orm(column_name = "signature")]
    pub signature: String,
    #[sea_orm(column_name = "download_url")]
    pub download_url: String,
    #[sea_orm(column_name = "release_notes", nullable)]
    pub release_notes: Option<String>,
    #[sea_orm(column_name = "is_critical")]
    pub is_critical: bool,
    #[sea_orm(column_name = "created_at", column_type = "Timestamp")]
    pub created_at: chrono::NaiveDateTime
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
