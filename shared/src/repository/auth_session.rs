use core_services::db::repository::abstraction::id::DbId;
use core_services::db::repository::abstraction::repository::Repository;
use core_services::db::repository::abstraction::table::Table;
use serde::{Deserialize, Serialize};

use crate::entities::session::{Session, SessionType};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuthSessionId {
    pub r#type: SessionType
}

pub trait AuthSessionRepository: Repository<Session, AuthSessionId> {}

impl Table<AuthSessionId> for Session {
    fn get_table() -> &'static str {
        "authSession"
    }

    fn id(&self) -> AuthSessionId {
        AuthSessionId {
            r#type: self.r#type.clone()
        }
    }
}

impl DbId for AuthSessionId {
    fn soft_deleted(&self) -> bool {
        false
    }

    fn soft_delete(&mut self) {
        self.r#type = SessionType::Access;
    }

    fn soft_restore(&mut self) {
        self.r#type = SessionType::Access;
    }
}
