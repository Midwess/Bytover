use std::future::Future;

use crux_core::capability::Operation;
use serde::{Deserialize, Serialize};

use crate::app::core::command::AppCommand;
use crate::app::AppRequestBuilder;
use crate::entities::local_resource::LocalResource;
use crate::entities::transfer_session::{ThumbnailUpdatedEvent, TransferProgress, TransferSession, TransferSessionStatus};
use crate::errors::CoreError;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TransferOperation {
    CreateCloudSession(TransferSession),
    SendSession(TransferSession),
    CancelSession(Option<String>, u64),
    FindSession {
        alias: String
    },
    SubscribeToPublicSessionTransferProgress {
        session_owner_user_id: u64,
        session_order_id: u64,
        password: Option<String>
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TransferOperationOutput {
    TransferResourceProgressUpdate(TransferProgress),
    TransferCompleted(TransferSessionStatus),
    PublicTransferSessionUpdated((Vec<LocalResource>, Vec<TransferProgress>)),
    SubscribeSessionEnded,
    ThumbnailUpdated(ThumbnailUpdatedEvent),
    SessionDetailReceived(schema::devlog::bitbridge::P2pTransferSessionMessage),
    ResourceSentToPeer {
        session_id: u64,
        resource_order_id: u64,
        peer_id: String
    }
}

impl Operation for TransferOperation {
    type Output = TransferOperationOutput;
}

impl TransferOperation {
    pub fn send_session(session: TransferSession) -> AppRequestBuilder<impl Future<Output = Result<(), CoreError>>> {
        AppCommand::request_from_shell(TransferOperation::SendSession(session)).map(|it| it.result())
    }

    pub fn cancel_session(peer_id: Option<String>, session_id: u64) -> AppRequestBuilder<impl Future<Output = Result<(), CoreError>>> {
        AppCommand::request_from_shell(TransferOperation::CancelSession(peer_id, session_id)).map(|it| it.result())
    }

    pub fn create_cloud_session(
        session: TransferSession
    ) -> AppRequestBuilder<impl Future<Output = Result<TransferSession, CoreError>>> {
        AppCommand::request_from_shell(TransferOperation::CreateCloudSession(session)).map(|it| it.result())
    }

    pub fn find_transfer_session(
        alias: String
    ) -> AppRequestBuilder<impl Future<Output = Result<Option<TransferSession>, CoreError>>> {
        AppCommand::request_from_shell(TransferOperation::FindSession { alias }).map(|it| it.result_option())
    }
}
