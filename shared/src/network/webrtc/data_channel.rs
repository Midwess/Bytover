use std::sync::Arc;
use webrtc::data_channel::RTCDataChannel;
use crate::{app::transfer::file_selection_service::ResourceSelection, ShellRuntime};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ResourceIdentifier {
    order_id: u64,
    local_path: String
}

impl ResourceIdentifier {
    pub fn serialize(&self) -> String {
        format!("file://{}:{}", self.order_id, self.local_path)
    }

    pub fn deserialize(s: &str) -> Result<Self, String> {
        let parts: Vec<&str> = s.split("://").collect();
        if parts.len() != 2 {
            return Err("Invalid format".to_string());
        }

        let order_id = parts[1].parse::<u64>().map_err(|_| "Invalid order ID".to_string())?;
        let local_path = parts[2].to_string();

        Ok(Self { order_id, local_path })
    }
}

// impl From<ResourceSelection> for ResourceIdentifier {
//     fn from(selection: ResourceSelection) -> Self {
//         match selection.path {
//             LocalResourcePath::LocalPath(path) => Self {
//                 order_id: 0,
//                 local_path: path
//             },
//             LocalResourcePath::PlatformIdentifier(identifier) => Self {
//                 order_id: 0,
//                 local_path: identifier
//             }
//         }
//     }
// }

// pub struct DataChannel {
//     channel: Arc<RTCDataChannel>,
//     shell_runtime: Arc<dyn ShellRuntime>,
// }

// impl DataChannel {
//     pub fn new(msg_channel: Arc<RTCDataChannel>) -> Self {
//         Self {
//             msg_channel
//         }
//     }

//     pub fn stream(&self)
// }