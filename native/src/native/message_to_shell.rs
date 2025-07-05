use crate::repository::path_resolver::{PathResolverMessage, PathResolverResponseMessage};
use serde::{Deserialize, Serialize};
use shared::app::operations::CoreOperationOutput;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MessageToShell {
    HandleResponse(u32, CoreOperationOutput),
    PathResolver(PathResolverMessage)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MessageToShellResponse {
    VoidResponse,
    PathResolverResponse(PathResolverResponseMessage)
}
