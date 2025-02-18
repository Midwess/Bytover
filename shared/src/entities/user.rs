use serde::{Deserialize, Serialize};
use surreal_derive_plus::SurrealDerive;
use uniffi::Record;

#[derive(Debug, Clone, Serialize, Deserialize, Record, PartialEq, Eq, SurrealDerive)]
pub struct User {
    pub email: String,
    pub name: String,
    pub avatar: String,
}