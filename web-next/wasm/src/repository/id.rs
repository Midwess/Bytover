use core_services::db::repository::abstraction::id::DbId;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IdbIdWrapper<T: Sized>(pub T)
where
    T: DbId + Debug;

impl<T: Sized> DbId for IdbIdWrapper<T>
where
    T: DbId + Debug
{
    fn soft_deleted(&self) -> bool {
        self.0.soft_deleted()
    }

    fn soft_delete(&mut self) {
        self.0.soft_delete();
    }

    fn soft_restore(&mut self) {
        self.0.soft_restore();
    }
}

impl<T: Sized> std::ops::Deref for IdbIdWrapper<T>
where
    T: DbId + Debug
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Sized> std::ops::DerefMut for IdbIdWrapper<T>
where
    T: DbId + Debug
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
