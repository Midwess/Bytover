use crate::entities::transfer_resource::TransferResourceType;
use crate::entities::transfer_session::TransferSession;
use schema::devlog::bitbridge::cloud_resource_message::ResourceType as CloudResourceType;
use schema::devlog::bitbridge::PublicTransferSessionMessage;

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
            order_id: value.order_id() as i64,
            user_id: value.user_order_id() as i64,
            access_url: value.access_url(),
            resources: vec![], // Todo
            password: value.password()
        }
    }
}
