use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub enum TransferMethodSelection {
    User(),
    Device(),
    Internet(),
}

impl Default for TransferMethodSelection {
    fn default() -> Self {
        Self::Device()
    }
}

pub struct TransferSelectionService {}

impl Default for TransferSelectionService {
    fn default() -> Self {
        Self::new()
    }
}

impl TransferSelectionService {
    pub fn new() -> Self {
        Self {}
    }
}
