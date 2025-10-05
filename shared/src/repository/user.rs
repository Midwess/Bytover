use crate::entities::user::User;
use core_services::db::repository::abstraction::id::DbId;
use core_services::db::repository::abstraction::repository::Repository;
use core_services::db::repository::abstraction::table::Table;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserId {
    pub email: String
}

pub trait UserRepository: Repository<User, UserId> {}

impl Table<UserId> for User {
    fn get_table() -> &'static str {
        "user"
    }

    fn id(&self) -> UserId {
        UserId { email: self.email.clone() }
    }
}

impl DbId for UserId {
    type Table = User;

    fn is_represent(&self, table: &Self::Table) -> bool {
        self.email == table.email
    }
}
