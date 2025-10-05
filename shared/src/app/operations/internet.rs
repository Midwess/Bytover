use std::future::Future;

use crux_core::capability::Operation;
use crux_core::Command;
use serde::{Deserialize, Serialize};

use crate::app::operations::device::GeoLocation;
use crate::app::AppRequestBuilder;
use crate::entities::finding_scope::FindingScope;
use crate::errors::NetworkError;

use super::{CoreOperation, CoreOperationOutput};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InternetOperation {
    Locate(Option<GeoLocation>)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum InternetOperationOutput {
    NetworkError(NetworkError),
    Locate(Vec<FindingScope>)
}

impl Operation for InternetOperation {
    type Output = InternetOperationOutput;
}

impl InternetOperation {
    pub fn locate(
        coordinate: Option<GeoLocation>
    ) -> AppRequestBuilder<impl Future<Output = Result<Vec<FindingScope>, NetworkError>>> {
        Command::request_from_shell(CoreOperation::Internet(InternetOperation::Locate(coordinate))).map(|it| match it {
            CoreOperationOutput::Internet(InternetOperationOutput::Locate(scopes)) => Ok(scopes),
            CoreOperationOutput::Internet(InternetOperationOutput::NetworkError(error)) => Err(error),
            _ => panic!("Mismatch in response type, expected GetCurrentIpAddress, got {it:?}")
        })
    }
}
