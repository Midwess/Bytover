use std::future::Future;

use crux_core::capability::Operation;
use crux_core::Command;
use schema::devlog::bitbridge::TransferSessionMessage;
use serde::{Deserialize, Serialize};

use crate::entities::finding_scope::FindingScope;
use crate::app::AppRequestBuilder;
use crate::entities::peer::Peer;
use crate::errors::NetworkError;

use super::{CoreOperation, CoreOperationOutput};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum P2POperation {
    StartNearbyServer(Peer),
    UpdateFindingScopes(Vec<FindingScope>),
    PeerEvents(String)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum P2POperationOutput {
    PeerConnected(Peer),
    PeerDisconnected(),
    ReceivedSessionRequest { remote_session: TransferSessionMessage },
    CancelSessionRequest { session_id: u64 },
    NearbyServerStopped
}

impl Operation for P2POperation {
    type Output = P2POperationOutput;
}

impl P2POperation {
    pub fn update_finding_scopes(scopes: Vec<FindingScope>) -> AppRequestBuilder<impl Future<Output = Result<(), NetworkError>>> {
        Command::request_from_shell(CoreOperation::P2P(P2POperation::UpdateFindingScopes(scopes))).map(|it| match it {
            CoreOperationOutput::Void => Ok(()),
            CoreOperationOutput::ConnectionError(e) => Err(e),
            _ => panic!("Mismatch in response type, expected Void, got {it:?}")
        })
    }
}
