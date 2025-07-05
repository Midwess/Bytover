use std::future::Future;

use crux_core::capability::Operation;
use crux_core::Command;
use serde::{Deserialize, Serialize};

use crate::app::AppRequestBuilder;
use crate::errors::NetworkError;

use super::{CoreOperation, CoreOperationOutput};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum InternetOperation {
    GetCurrentIpAddress
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum InternetOperationOutput {
    NetworkError(NetworkError),
    GetCurrentIpAddress(String)
}

impl Operation for InternetOperation {
    type Output = InternetOperationOutput;
}

impl InternetOperation {
    pub fn get_current_ip_address() -> AppRequestBuilder<impl Future<Output = Result<String, NetworkError>>> {
        Command::request_from_shell(CoreOperation::Internet(InternetOperation::GetCurrentIpAddress)).map(|it| match it {
            CoreOperationOutput::Internet(InternetOperationOutput::GetCurrentIpAddress(ip)) => Ok(ip),
            CoreOperationOutput::Internet(InternetOperationOutput::NetworkError(error)) => Err(error),
            _ => panic!("Mismatch in response type, expected GetCurrentIpAddress, got {it:?}")
        })
    }
}
