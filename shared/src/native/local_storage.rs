use crate::app::operations::local_storage::{LocalStorageOperation, LocalStorageOperationOutput};

pub struct NativeLocalStorage {}

impl NativeLocalStorage {
    pub async fn handle(&self, effect: LocalStorageOperation) -> LocalStorageOperationOutput {
        match effect {
            LocalStorageOperation::NewFile { bytes, path } => {
                todo!()
            }
            LocalStorageOperation::Copy { source, destination } => {
                todo!()
            }
            LocalStorageOperation::Zip { source, destination } => {
                todo!()
            }
            LocalStorageOperation::Get { path } => {
                todo!()
            }
            _ => {
                panic!("Unsupported operation: {:?}", effect)
            }
        }
    }
}
