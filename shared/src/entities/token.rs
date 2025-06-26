use serde::{Deserialize, Serialize};

use uniffi::Record;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Record)]
pub struct Token {
    pub order_id: u64,
    pub value: String
}
