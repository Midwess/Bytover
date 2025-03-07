use serde::{Deserialize, Serialize};
use uniffi::{Enum, Record};

#[derive(Debug, PartialEq, Enum, Serialize, Deserialize, Clone)]
pub enum TransferMethodSelection {
    User(),
    Device(),
    Internet()
}

impl Default for TransferMethodSelection {
    fn default() -> Self {
        Self::Device()
    }
}

pub struct TransferSelectionService {

}

impl TransferSelectionService {
    pub fn new() -> Self {
        Self {}
    }
}
