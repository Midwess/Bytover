use serde::{Deserialize, Serialize};

use uniffi::Record;

#[derive(Debug, Clone, Serialize, Deserialize, Record, PartialEq, Eq)]
pub struct User {
    pub email: String,
    pub name: String,
    pub avatar: String
}
