use std::future::Future;

use crux_core::capability::Operation;
use crux_core::Command;
use serde::{Deserialize, Serialize};

use crate::app::AppRequestBuilder;
use crate::entities::finding_scope::FindingScope;
use crate::entities::local_resource::LocalResource;
use crate::entities::peer::Peer;
use crate::entities::transfer_session::{TransferProgress, TransferSession};
use crate::errors::CoreError;
use schema::devlog::rpc_signalling::server::ScopeState;

use super::CoreOperation;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum P2POperation {
    StartNearbyServer(Peer),
    StopNearbyServer,
    UpdateFindingScopes(Vec<FindingScope>),
    PeerEvents(String),
    IsRunning,
    ViewSessionDetail {
        peer_id: String,
        order_id: u64,
        password: Option<String>
    },
    SendSessionDetail {
        peer_id: String,
        request_id: String,
        session_message: Option<schema::devlog::bitbridge::P2pTransferSessionMessage>,
        resources: Option<Vec<LocalResource>>,
        error: Option<CoreError>
    },
    DownloadResource {
        peer_id: String,
        session_id: u64,
        resource: LocalResource,
        progress: TransferProgress
    },
    StreamResourceToPeer {
        peer_id: String,
        session_id: u64,
        transfer_id: u16,
        resource: LocalResource
    },
    CancelResource {
        peer_id: String,
        session_id: u64,
        resource_id: u64
    },
    BroadcastCancelSession {
        session_id: u64,
        resource_id: Option<u64>
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum P2POperationOutput {
    PeerConnected(Peer),
    PeerDisconnected(),
    PeerScopesUpdated(Vec<FindingScope>),
    CancelSessionRequest { session_id: u64 },
    NearbyServerStopped,
    AlreadyRunning,
    ReceivedViewSessionRequest {
        peer_id: String,
        request_id: String,
        order_id: u64,
        password: Option<String>
    },
    SessionDetailReceived {
        session: TransferSession
    },
    SessionDetailFailed {
        order_id: u64,
        error: String
    },
    ReceivedDownloadRequest {
        peer_id: String,
        session_order_id: u64,
        resource_order_id: u64,
        transfer_id: u16
    },
    ReceivedResourceNotification {
        session_order_id: u64,
        resource: LocalResource,
        peer_id: String,
    },
    ScopeStateChanged {
        scope_id: String,
        state: ScopeState,
    }
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

    pub fn view_session_detail(peer_id: String, order_id: u64, password: Option<String>) -> AppRequestBuilder<impl Future<Output = Result<(), CoreError>>> {
        Command::request_from_shell(CoreOperation::P2P(P2POperation::ViewSessionDetail { peer_id, order_id, password })).map(|it| it.result())
    }

    pub fn send_session_detail(
        peer_id: String,
        request_id: String,
        session_message: Option<schema::devlog::bitbridge::P2pTransferSessionMessage>,
        resources: Option<Vec<LocalResource>>
    ) -> AppRequestBuilder<impl Future<Output = Result<(), CoreError>>> {
        Command::request_from_shell(CoreOperation::P2P(P2POperation::SendSessionDetail {
            peer_id,
            request_id,
            session_message,
            resources,
            error: None
        })).map(|it| it.result())
    }

    pub fn send_session_detail_error(peer_id: String, request_id: String, error: CoreError) -> AppRequestBuilder<impl Future<Output = Result<(), CoreError>>> {
        Command::request_from_shell(CoreOperation::P2P(P2POperation::SendSessionDetail {
            peer_id,
            request_id,
            session_message: None,
            resources: None,
            error: Some(error)
        })).map(|it| it.result())
    }

    pub fn stream_resource_to_peer(peer_id: String, session_id: u64, transfer_id: u16, resource: LocalResource) -> AppRequestBuilder<impl Future<Output = Result<(), CoreError>>> {
        Command::request_from_shell(CoreOperation::P2P(P2POperation::StreamResourceToPeer { peer_id, session_id, transfer_id, resource })).map(|it| it.result())
    }

    pub fn cancel_resource(peer_id: String, session_id: u64, resource_id: u64) -> AppRequestBuilder<impl Future<Output = Result<(), CoreError>>> {
        Command::request_from_shell(CoreOperation::P2P(P2POperation::CancelResource { peer_id, session_id, resource_id })).map(|it| it.result())
    }

    pub fn broadcast_cancel_session(session_id: u64, resource_id: Option<u64>) -> AppRequestBuilder<impl Future<Output = Result<(), CoreError>>> {
        Command::request_from_shell(CoreOperation::P2P(P2POperation::BroadcastCancelSession { session_id, resource_id })).map(|it| it.result())
    }
}
