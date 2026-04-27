use crate::db::repository::abstraction::table::Table;

pub trait DbId: Sized {
    type Table: Table<Self>;

    fn soft_deleted(&self) -> bool {
        false
    }

    fn soft_delete(&mut self) {}
    fn soft_restore(&mut self) {}

    fn is_represent(&self, _: &Self::Table) -> bool {
        false
    }
}

/// Extension trait: lookup on Vec<Table<I>> by &I (the DbId)
pub trait VecTableLookup<I: DbId> {
    fn lookup(&self, id: &I) -> Option<&I::Table>;
    fn lookup_mut(&mut self, id: &I) -> Option<&mut I::Table>;
    fn lookup_all<'a>(&'a self, id: &'a I) -> impl Iterator<Item = &'a I::Table>;
    fn lookup_all_mut<'a>(&'a mut self, id: &'a I) -> impl Iterator<Item = &'a mut I::Table>;
}

impl<I> VecTableLookup<I> for Vec<I::Table>
where
    I: DbId
{
    fn lookup(&self, id: &I) -> Option<&I::Table> {
        self.iter().find(|t| id.is_represent(t))
    }

    fn lookup_mut(&mut self, id: &I) -> Option<&mut I::Table> {
        self.iter_mut().find(|t| id.is_represent(t))
    }

    fn lookup_all<'a>(&'a self, id: &'a I) -> impl Iterator<Item = &'a I::Table> {
        self.iter().filter(move |t| id.is_represent(t))
    }

    fn lookup_all_mut<'a>(&'a mut self, id: &'a I) -> impl Iterator<Item = &'a mut I::Table> {
        self.iter_mut().filter(move |t| id.is_represent(t))
    }
}
