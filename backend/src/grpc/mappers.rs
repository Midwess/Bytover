use crate::entities::transfer_resource::{TransferResource, TransferResourceType};
use crate::entities::transfer_session::TransferSession;
use schema::devlog::bitbridge::cloud_resource_message::ResourceType as CloudResourceType;
use schema::devlog::bitbridge::{CloudResourceMessage, PublicTransferSessionMessage, ResourceTypeMessage};
use schema::devlog::bitbridge::public_transfer_session_message::Progress;
use crate::entities::transfer_progress::TransferProgress;

impl From<&CloudResourceType> for TransferResourceType {
    fn from(value: &CloudResourceType) -> Self {
        match value {
            CloudResourceType::Image => Self::Image,
            CloudResourceType::Video => Self::Video,
            CloudResourceType::File => Self::File,
            CloudResourceType::Folder => Self::Folder
        }
    }
}

impl From<TransferSession> for PublicTransferSessionMessage {
    fn from(value: TransferSession) -> Self {
        Self {
            order_id: value.order_id(),
            user_id: value.user_order_id(),
            access_url: value.access_url(),
            resources: value.resources().iter().map(|it| it.clone().into()).collect(),
            password: value.password(),
            progresses: value.progresses().iter().map(|it| it.clone().into()).collect(),
        }
    }
}

impl From<TransferProgress> for Progress {
    fn from(value: TransferProgress) -> Self {
        Self {
            resource_order_id: value.resource_id(),
            completion: value.completion(),
            error_message: value.error_message().map(|it| it.to_owned()),
        }
    }
}

impl From<TransferResource> for CloudResourceMessage {
    fn from(value: TransferResource) -> Self {
        Self {
            r#type: Into::<ResourceTypeMessage>::into(value.r#type()).into(),
            name: value.name().to_owned(),
            order_id: value.order_id(),
            size: value.size_in_bytes() as i64,
        }
    }
}

impl From<TransferResourceType> for ResourceTypeMessage {
    fn from(value: TransferResourceType) -> Self {
        match value {
            TransferResourceType::File => {
                Self::File
            }
            TransferResourceType::Folder => {
                Self::Folder
            }
            TransferResourceType::Image => {
                Self::Image
            }
            TransferResourceType::Video => {
                Self::Video
            }
        }
    }
}