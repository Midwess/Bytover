use core_services::db::repository::abstraction::id::DbId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RedbIdWrapper<T: Sized>(pub T);

impl<T: Sized> DbId for RedbIdWrapper<T>
where
    T: DbId
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

impl<T: Sized> std::ops::Deref for RedbIdWrapper<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Sized> std::ops::DerefMut for RedbIdWrapper<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
