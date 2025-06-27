use serde::{Deserialize, Serialize};
use shared::app::file_system::file::LocalResourcePath;
use shared::app::operations::CoreOperationOutput;
use uniffi::Enum;

#[derive(Debug, Clone, Enum, PartialEq, Serialize, Deserialize)]
pub enum MessageToShell {
    HandleResponse(u32, CoreOperationOutput),
    ResolveAbsolutePath(LocalResourcePath),
    ResolveLocalResourcePath(String)
}

#[derive(Debug, Clone, Enum, PartialEq, Serialize, Deserialize)]
pub enum MessageToShellResponse {
    VoidResponse,
    ResolveAbsolutePath(Option<String>),
    ResolveLocalResourcePath(Option<LocalResourcePath>)
}
