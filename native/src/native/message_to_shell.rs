use serde::{Deserialize, Serialize};
use uniffi::Enum;

use shared::app::operations::CoreOperationOutput;

#[derive(Debug, Clone, Enum, PartialEq, Serialize, Deserialize)]
pub enum MessageToShell {
    HandleResponse(u32, CoreOperationOutput)
}
