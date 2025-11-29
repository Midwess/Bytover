use std::future::Future;

use crux_core::capability::Operation;
use crux_core::Command;
use schema::devlog::bitbridge::TransferSessionMessage;
use serde::{Deserialize, Serialize};

use crate::app::AppRequestBuilder;
use crate::entities::finding_scope::FindingScope;
use crate::entities::peer::Peer;
use crate::errors::CoreError;

use super::CoreOperation;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum P2POperation {
    StartNearbyServer(Peer),
    StopNearbyServer,
    UpdateFindingScopes(Vec<FindingScope>),
    PeerEvents(String),
    IsRunning
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum P2POperationOutput {
    PeerConnected(Peer),
    PeerDisconnected(),
    ReceivedSessionRequest { remote_session: TransferSessionMessage },
    CancelSessionRequest { session_id: u64 },
    // Happily stopped
    // if error happened, it will be OperationOutput::Error(CoreError)
    NearbyServerStopped
}

impl Operation for P2POperation {
    type Output = P2POperationOutput;
}

impl P2POperation {
    pub fn update_finding_scopes(scopes: Vec<FindingScope>) -> AppRequestBuilder<impl Future<Output = Result<(), CoreError>>> {
        Command::request_from_shell(CoreOperation::P2P(P2POperation::UpdateFindingScopes(scopes))).map(|it| it.result())
    }

    pub fn stop() -> AppRequestBuilder<impl Future<Output = Result<(), CoreError>>> {
        Command::request_from_shell(CoreOperation::P2P(P2POperation::StopNearbyServer)).map(|it| it.result())
    }

    pub fn start(peer: Peer) -> AppRequestBuilder<impl Future<Output = Result<(), CoreError>>> {
        Command::request_from_shell(CoreOperation::P2P(P2POperation::StartNearbyServer(peer))).map(|it| it.result())
    }

    pub fn is_running() -> AppRequestBuilder<impl Future<Output = Result<bool, CoreError>>> {
        Command::request_from_shell(CoreOperation::P2P(P2POperation::IsRunning)).map(|it| it.result())
    }
}
