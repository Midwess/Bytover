use devlog_sdk::distributed_id::gen_id;
use schema::value::static_resource::static_resource::Source;
use schema::value::static_resource::{S3Path, StaticResource};
use serde::{Deserialize, Serialize};
use surreal_derive_plus::SurrealDerive;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, SurrealDerive)]
pub enum TransferResourceType {
    File,
    Folder,
    Image,
    Video
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, SurrealDerive)]
pub struct TransferResource {
    order_id: u64,
    name: String,
    size: u64,
    source: StaticResource,
    thumbnail_path: Option<S3Path>,
    r#type: TransferResourceType
}

impl TransferResource {
    pub async fn new(
        order_id: Option<u64>,
        session_id: u64,
        name: impl Into<String>,
        size: u64,
        r#type: TransferResourceType
    ) -> Self {
        let name = name.into();
        Self {
            order_id: order_id.unwrap_or(gen_id().await),
            name: name.clone(),
            size,
            r#type,
            thumbnail_path: None,
            source: StaticResource::s3_path(S3Path::use_default_bucket(format!("bitbridge/sessions/{session_id}/{name}")))
        }
    }

    pub fn order_id(&self) -> u64 {
        self.order_id
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn size_in_bytes(&self) -> u64 {
        self.size
    }

    pub fn source(&self) -> &StaticResource {
        &self.source
    }

    pub fn source_mut(&mut self) -> &mut StaticResource {
        &mut self.source
    }

    pub fn set_public_url(&mut self, public_url: impl Into<String>) {
        self.source = StaticResource {
            source: Some(Source::Url(public_url.into()))
        }
    }

    pub fn thumbnail_source(&self, session_id: u64) -> Option<StaticResource> {
        if matches!(self.r#type, TransferResourceType::Folder) {
            return None
        }

        Some(StaticResource::s3_path(S3Path::use_default_bucket(format!(
            "bitbridge/thumbnails/sessions/{session_id}/{}.png",
            self.order_id
        ))))
    }
}
