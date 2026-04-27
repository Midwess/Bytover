use crate::db::repository::abstraction::id::DbId;
use crate::db::repository::abstraction::repository::SendSync;

pub trait RedbId: DbId + SendSync + std::fmt::Debug {
    fn lower_id(&self) -> Vec<Vec<u8>>;
    fn upper_id(&self) -> Vec<Vec<u8>> {
        self.lower_id()
            .into_iter()
            .map(|part| {
                if part.iter().all(|&b| b == 0) {
                    // replace all-zero part with max bytes
                    Vec::<u8>::from(vec![0xFF; part.len()])
                } else {
                    part
                }
            })
            .collect()
    }
}
