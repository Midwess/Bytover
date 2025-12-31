use crate::entities::local_resource::{LocalResource, LocalResourcePath, ResourceType};
use crate::entities::transfer_session::{TransferProgress, TransferType};
use schema::devlog::bitbridge::public_transfer_session_message::Progress;
use schema::devlog::bitbridge::{cloud_resource_message, CloudResourceMessage, ResourceMessage, ResourceTypeMessage};

impl From<Progress> for TransferProgress {
    fn from(value: Progress) -> Self {
        let mut this = Self::new(value.resource_order_id, value.total_size, TransferType::Receive);

        this.update_progress(value.transfered_amount);
        this
    }
}

impl From<cloud_resource_message::ResourceType> for ResourceType {
    fn from(value: cloud_resource_message::ResourceType) -> Self {
        match value {
            cloud_resource_message::ResourceType::Image => Self::Image,
            cloud_resource_message::ResourceType::Video => Self::Video,
            cloud_resource_message::ResourceType::File => Self::File,
            cloud_resource_message::ResourceType::Folder => Self::Folder
        }
    }
}

impl From<CloudResourceMessage> for LocalResource {
    fn from(value: CloudResourceMessage) -> Self {
        Self {
            order_id: value.order_id,
            name: value.name,
            size: value.size as u64,
            path: LocalResourcePath::AbsolutePath(value.download_url),
            thumbnail_path: value.thumbnail_download_url.map(LocalResourcePath::AbsolutePath),
            r#type: cloud_resource_message::ResourceType::try_from(value.r#type)
                .unwrap_or(cloud_resource_message::ResourceType::File)
                .into()
        }
    }
}

impl LocalResource {
    pub fn to_proto(&self) -> ResourceMessage {
        let proto_type = match self.r#type {
            ResourceType::Image => ResourceTypeMessage::Image,
            ResourceType::Video => ResourceTypeMessage::Video,
            ResourceType::File => ResourceTypeMessage::File,
            ResourceType::Folder => ResourceTypeMessage::Folder
        };

        ResourceMessage {
            order_id: self.order_id,
            name: self.name.clone(),
            size: self.size as i64,
            r#type: proto_type as i32,
            thumbnail_png: None
        }
    }
}

impl From<ResourceTypeMessage> for ResourceType {
    fn from(value: ResourceTypeMessage) -> Self {
        match value {
            ResourceTypeMessage::Image => ResourceType::Image,
            ResourceTypeMessage::Video => ResourceType::Video,
            ResourceTypeMessage::File => ResourceType::File,
            ResourceTypeMessage::Folder => ResourceType::Folder
        }
    }
}
