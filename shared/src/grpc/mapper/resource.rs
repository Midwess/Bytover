use crate::app::file_system::file::{LocalResource, ResourceType};
use schema::devlog::bitbridge::{ResourceMessage, ResourceTypeMessage};

impl LocalResource {
    pub async fn to_proto(&self) -> ResourceMessage {
        let proto_type = match self.r#type {
            ResourceType::Image => ResourceTypeMessage::Image,
            ResourceType::Video => ResourceTypeMessage::Video,
            ResourceType::File => ResourceTypeMessage::File,
        };

        ResourceMessage {
            order_id: self.order_id as i64,
            name: self.name.clone(),
            size: self.size as i64,
            thumbnail_png: None,
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
        }
    }
}
