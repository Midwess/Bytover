use schema::value::static_resource::S3Path;
use serde::{Deserialize, Serialize};
use surreal_derive_plus::SurrealDerive;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, SurrealDerive)]
pub enum TransferResourceType {
    File,
    Directory,
    Image,
    Video
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, SurrealDerive)]
pub struct TransferResource {
    order_id: u64,
    name: String,
    size: u64,
    path: S3Path,
    thumbnail_path: Option<S3Path>,
    r#type: TransferResourceType
}

impl TransferResource {
    pub fn order_id(&self) -> u64 {
        self.order_id
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn size_in_bytes(&self) -> u64 {
        self.size
    }

    pub fn saved_path(&self) -> &S3Path {
        &self.path
    }
}
