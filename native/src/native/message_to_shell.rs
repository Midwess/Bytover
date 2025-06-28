use serde::{Deserialize, Serialize};
use shared::app::operations::CoreOperationOutput;
use uniffi::Enum;
use crate::repository::path_resolver::{PathResolverMessage, PathResolverResponseMessage};

#[derive(Debug, Clone, Enum, PartialEq, Serialize, Deserialize)]
pub enum MessageToShell {
    HandleResponse(u32, CoreOperationOutput),
    PathResolver(PathResolverMessage),
}

#[derive(Debug, Clone, Enum, PartialEq, Serialize, Deserialize)]
pub enum MessageToShellResponse {
    VoidResponse,
    PathResolverResponse(PathResolverResponseMessage)
}
