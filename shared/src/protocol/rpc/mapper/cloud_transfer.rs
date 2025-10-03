use crate::entities::local_resource::ResourceType;
use schema::devlog::bitbridge::cloud_resource_message::ResourceType as SchemaResourceType;

impl From<&ResourceType> for SchemaResourceType {
    fn from(value: &ResourceType) -> Self {
        match value {
            ResourceType::File => SchemaResourceType::File,
            ResourceType::Image => SchemaResourceType::Image,
            ResourceType::Video => SchemaResourceType::Video,
            ResourceType::Folder => SchemaResourceType::Folder
        }
    }
}
