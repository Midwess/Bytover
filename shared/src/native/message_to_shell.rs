use serde::{Deserialize, Serialize};
use uniffi::Enum;

use crate::entities::user::User;

#[derive(Debug, Clone, Enum, PartialEq, Serialize, Deserialize)]
pub enum MessageToShell {
    NewNearby { address: String, user: User },
    NearbyRemoved { address: String, user: User }
}
