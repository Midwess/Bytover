use serde::{Deserialize, Serialize};
use surreal_derive_plus::SurrealDerive;
use uniffi::Record;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, SurrealDerive, Record)]
pub struct Token {
    pub id: u64,
    pub value: String
}