use crate::entities::transfer_resource::TransferResourceType;
use schema::devlog::bitbridge::cloud_resource_message::ResourceType as CloudResourceType;

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
