use serde::{Deserialize, Serialize};
use surreal_derive_plus::SurrealDerive;
use uniffi::{Enum, Record};

use super::token::Token;
use super::user::User;

#[derive(Debug, Clone, Serialize, Deserialize, Enum, PartialEq, Eq, SurrealDerive)]
pub enum SessionType {
    Access
}

#[derive(Debug, Clone, Serialize, Deserialize, Record, PartialEq, Eq, SurrealDerive)]
pub struct Session {
    pub user: Option<User>,
    pub token: Token,
    pub r#type: SessionType
}
