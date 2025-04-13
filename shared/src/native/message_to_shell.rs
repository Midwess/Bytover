use serde::{Deserialize, Serialize};
use uniffi::Enum;

use crate::app::operations::CoreOperationOutput;

#[derive(Debug, Clone, Enum, PartialEq, Serialize, Deserialize)]
pub enum MessageToShell {
    HandleResponse(u32, CoreOperationOutput)
}
