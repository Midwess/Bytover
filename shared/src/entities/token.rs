use serde::{Deserialize, Serialize};
use surreal_derive_plus::SurrealDerive;
use uniffi::Record;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, SurrealDerive, Record)]
pub struct Token {
    pub order_id: u64,
    pub value: String
}