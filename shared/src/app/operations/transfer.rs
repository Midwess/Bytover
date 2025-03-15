use std::future::Future;

use crux_core::capability::Operation;
use crux_core::Command;
use serde::{Deserialize, Serialize};

use crate::app::AppRequestBuilder;

use super::{CoreOperation, CoreOperationOutput};

/// This operation is used to access the local storage of device.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TransferOperation {
    StartNearbyServer,
    StopNearbyServer
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TransferOperationOutput {
    StartNearbyServer,
    StopNearbyServer
}

impl Operation for TransferOperation {
    type Output = TransferOperationOutput;
}

impl TransferOperation {
    pub fn start_nearby_server() -> AppRequestBuilder<impl Future<Output = ()>> {
        Command::request_from_shell(CoreOperation::Transfer(TransferOperation::StartNearbyServer)).map(|it| match it {
            CoreOperationOutput::Transfer(TransferOperationOutput::StartNearbyServer) => (),
            _ => panic!("Mismatch in response type, expected StartNearbyServer, got {:?}", it)
        })
    }

    pub fn stop_nearby_server() -> AppRequestBuilder<impl Future<Output = ()>> {
        Command::request_from_shell(CoreOperation::Transfer(TransferOperation::StopNearbyServer)).map(|it| match it {
            CoreOperationOutput::Transfer(TransferOperationOutput::StopNearbyServer) => (),
            _ => panic!("Mismatch in response type, expected StopNearbyServer, got {:?}", it)
        })
    }
}
