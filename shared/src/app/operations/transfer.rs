use std::future::Future;

use crux_core::capability::Operation;
use crux_core::Command;
use serde::{Deserialize, Serialize};

use crate::app::{file_system::file::LocalResource, transfer::finding_scope::FindingScope};
use crate::app::AppRequestBuilder;
use crate::entities::peer::Peer;

use super::{CoreOperation, CoreOperationOutput};

/// This operation is used to access the local storage of device.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TransferOperation {
    StartNearbyServer(Peer),
    StopNearbyServer,
    UpdateFindingScopes(Vec<FindingScope>),
    // Transfer((Vec<LocalResource>, Peer)),
    // TransferProgressUpdate()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TransferOperationOutput {
    StartNearbyServer,
    StopNearbyServer,
    UpdateFindingScopes
}

impl Operation for TransferOperation {
    type Output = TransferOperationOutput;
}

impl TransferOperation {
    pub fn start_nearby_server(peer: Peer) -> AppRequestBuilder<impl Future<Output = ()>> {
        Command::request_from_shell(CoreOperation::Transfer(TransferOperation::StartNearbyServer(peer))).map(|it| match it {
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

    pub fn update_finding_scopes(scopes: Vec<FindingScope>) -> AppRequestBuilder<impl Future<Output = ()>> {
        Command::request_from_shell(CoreOperation::Transfer(TransferOperation::UpdateFindingScopes(scopes))).map(|it| match it {
            CoreOperationOutput::Transfer(TransferOperationOutput::UpdateFindingScopes) => (),
            _ => panic!("Mismatch in response type, expected UpdateFindingScopes, got {:?}", it)
        })
    }
}
