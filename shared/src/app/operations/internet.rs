use std::future::Future;

use serde::{Deserialize, Serialize};

use super::CoreError;
use crate::app::core::command::AppCommand;
use crate::app::operations::device::GeoLocation;
use crate::app::AppRequestBuilder;
use crate::entities::finding_scope::FindingScope;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InternetOperation {
    Locate(Option<GeoLocation>)
}

impl InternetOperation {
    pub fn locate(coordinate: Option<GeoLocation>) -> AppRequestBuilder<impl Future<Output = Result<Vec<FindingScope>, CoreError>>> {
        AppCommand::request_from_shell(InternetOperation::Locate(coordinate)).map(|it| it.result())
    }
}
