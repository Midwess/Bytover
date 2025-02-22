use crux_core::capability::Operation;
use serde::{Deserialize, Serialize};

/// This operation is used to access the local storage of device.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TransferOperation {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TransferOperationOutput {}

impl Operation for TransferOperation {
    type Output = TransferOperationOutput;
}
