use std::future::Future;

use crux_core::{capability::Operation, Command};
use schema::devlog::bitbridge::{peer_message_body::Request, PeerMessageBody, TransferSessionMessage};
use serde::{Deserialize, Serialize};
use uniffi::{Enum, Record};

use crate::{app::{nearby::finding_scope::FindingScope, AppRequestBuilder}, entities::peer::Peer, errors::NetworkError};

use super::{CoreOperation, CoreOperationOutput};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Enum)]
pub enum P2POperation {
    StartNearbyServer(Peer),
    UpdateFindingScopes(Vec<FindingScope>),
    PeerEvents(String)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Enum)]
pub enum P2POperationOutput {
    PeerConnected(Peer),
    PeerDisconnected(),
    ReceivedSessionRequest {
        request_id: String,
        remote_session: TransferSessionMessage,
    },
    NearbyServerStopped,
}

impl Operation for P2POperation {
    type Output = P2POperationOutput;
}

impl P2POperation {
    pub fn update_finding_scopes(scopes: Vec<FindingScope>) -> AppRequestBuilder<impl Future<Output = Result<(), NetworkError>>> {
        Command::request_from_shell(CoreOperation::P2P(P2POperation::UpdateFindingScopes(scopes))).map(|it| match it {
            CoreOperationOutput::Void => Ok(()),
            CoreOperationOutput::ConnectionError(e) => Err(e),
            _ => panic!("Mismatch in response type, expected Void, got {:?}", it)
        })
    }
}