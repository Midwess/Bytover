use std::future::Future;

use crux_core::capability::Operation;
use crux_core::Command;
use serde::{Deserialize, Serialize};

use crate::app::operations::device::GeoLocation;
use crate::app::AppRequestBuilder;
use crate::app::core::command::AppCommand;
use crate::entities::finding_scope::FindingScope;
use crate::errors::NetworkError;

use super::{CoreError, CoreOperation, CoreOperationOutput};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InternetOperation {
    Locate(Option<GeoLocation>)
}

impl InternetOperation {
    pub fn locate(
        coordinate: Option<GeoLocation>
    ) -> AppRequestBuilder<impl Future<Output = Result<Vec<FindingScope>, CoreError>>> {
        AppCommand::request_from_shell(InternetOperation::Locate(coordinate)).map(|it| it.result())
    }
}
