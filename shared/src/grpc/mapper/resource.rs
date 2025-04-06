use crate::app::file_system::file::{LocalResource, LocalResourcePath, ResourceType};
use schema::devlog::bitbridge::{ResourceMessage, ResourceTypeMessage};
use tokio::fs;

impl LocalResource {
    pub async fn to_proto(&self) -> ResourceMessage {
        let proto_type = match self.r#type {
            ResourceType::Image => ResourceTypeMessage::Image,
            ResourceType::Video => ResourceTypeMessage::Video,
            ResourceType::File => ResourceTypeMessage::File,
            ResourceType::Folder => ResourceTypeMessage::Folder,
            ResourceType::Other => ResourceTypeMessage::Other
        };

        let thumbnail_png = match &self.thumbnail_path {
            Some(LocalResourcePath::LocalPath(path)) => (fs::read(path).await).ok(),
            Some(LocalResourcePath::PlatformIdentifier(_)) => None,
            None => None
        };

        ResourceMessage {
            order_id: self.order_id as i64,
            name: self.name.clone(),
            size: self.size as i64,
            thumbnail_png,
            r#type: proto_type as i32
        }
    }
}

impl From<ResourceTypeMessage> for ResourceType {
    fn from(value: ResourceTypeMessage) -> Self {
        match value {
            ResourceTypeMessage::Image => ResourceType::Image,
            ResourceTypeMessage::Video => ResourceType::Video,
            ResourceTypeMessage::File => ResourceType::File,
            ResourceTypeMessage::Folder => ResourceType::Folder,
            ResourceTypeMessage::Other => ResourceType::Other
        }
    }
}
