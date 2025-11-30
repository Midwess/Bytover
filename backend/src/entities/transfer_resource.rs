use devlog_sdk::distributed_id::gen_id;
use schema::value::static_resource::{S3Path, StaticResource};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TransferResourceType {
    File,
    Folder,
    Image,
    Video
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransferResource {
    order_id: u64,
    session_id: u64,
    name: String,
    size: u64,
    #[serde(rename = "r#type")]
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
        let order_id = order_id.unwrap_or(gen_id().await);

        Self {
            name: name.clone(),
            order_id,
            session_id,
            size,
            r#type
        }
    }

    pub fn order_id(&self) -> u64 {
        self.order_id
    }

    pub fn r#type(&self) -> TransferResourceType {
        self.r#type.clone()
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn size_in_bytes(&self) -> u64 {
        self.size
    }

    pub fn source(&self) -> StaticResource {
        if matches!(self.r#type, TransferResourceType::Folder) {
            let name = self.name.trim_end_matches(".tar").trim_end_matches(".zip").trim_end_matches(".rar");
            return StaticResource::s3_path(S3Path::use_default_bucket(format!(
                "sessions/{}/{}.zip",
                self.session_id, name
            )))
        }

        StaticResource::s3_path(S3Path::use_default_bucket(format!(
            "sessions/{}/{}",
            self.session_id, self.name
        )))
    }

    pub fn thumbnail_source(&self) -> Option<StaticResource> {
        if matches!(self.r#type, TransferResourceType::Folder) {
            return None
        }

        Some(StaticResource::s3_path(S3Path::use_default_bucket(format!(
            "thumbnails/sessions/{}/{}.png",
            self.session_id, self.order_id
        ))))
    }
}
