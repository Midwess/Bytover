use crate::repository::path_resolver::{PathResolverMessage, PathResolverResponseMessage};
use serde::{Deserialize, Serialize};
use shared::app::operations::CoreOperationOutput;
use shared::app::AppEvent;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MessageToShell {
    HandleResponse(u32, Box<CoreOperationOutput>),
    PathResolver(PathResolverMessage),
    Notify(Box<AppEvent>)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MessageToShellResponse {
    VoidResponse,
    PathResolverResponse(PathResolverResponseMessage)
}
