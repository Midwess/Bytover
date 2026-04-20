use crate::entities::local_resource::{LocalResource, LocalResourcePath, ResourceType};
use crate::entities::transfer_session::{TransferProgress, TransferStatus, TransferType};
use schema::devlog::bitbridge::public_transfer_session_message::Progress;
use schema::devlog::bitbridge::{cloud_resource_message, CloudResourceMessage, ResourceMessage, ResourceTypeMessage};

impl From<Progress> for TransferProgress {
    fn from(value: Progress) -> Self {
        let mut this = Self::new(value.resource_order_id, value.total_size, TransferType::Receive);

        if let Some(status) = value.status {
            this.status = match status {
                1 => TransferStatus::Success,
                2 => TransferStatus::Fail(value.error_message.unwrap_or_default()),
                3 => TransferStatus::Canceled,
                _ => infer_status_from_legacy_fields(&value),
            };
        } else {
            this.status = infer_status_from_legacy_fields(&value);
        }

        this.update_progress(value.transfered_amount);
        this
    }
}

fn infer_status_from_legacy_fields(progress: &Progress) -> TransferStatus {
    if let Some(ref error) = progress.error_message {
        if !error.is_empty() {
            return TransferStatus::Fail(error.clone());
        }
    }
    if progress.transfered_amount >= progress.total_size && progress.total_size > 0 {
        TransferStatus::Success
    } else {
        TransferStatus::InProgress
    }
}

impl From<cloud_resource_message::ResourceType> for ResourceType {
    fn from(value: cloud_resource_message::ResourceType) -> Self {
        match value {
            cloud_resource_message::ResourceType::Image => Self::Image,
            cloud_resource_message::ResourceType::Video => Self::Video,
            cloud_resource_message::ResourceType::File => Self::File,
            cloud_resource_message::ResourceType::Folder => Self::Folder,
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
                .into(),
            shelf_id: 0,
        }
    }
}

impl LocalResource {
    pub fn to_proto(&self) -> ResourceMessage {
        let proto_type = match self.r#type {
            ResourceType::Image => ResourceTypeMessage::Image,
            ResourceType::Video => ResourceTypeMessage::Video,
            ResourceType::File => ResourceTypeMessage::File,
            ResourceType::Folder => ResourceTypeMessage::Folder,
        };

        ResourceMessage {
            order_id: self.order_id,
            name: self.name.clone(),
            size: self.size as i64,
            r#type: proto_type as i32,
            thumbnail_png: None,
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
        }
    }
}
