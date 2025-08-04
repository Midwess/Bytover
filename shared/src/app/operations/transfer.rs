use std::future::Future;

use crux_core::capability::Operation;
use crux_core::Command;
use serde::{Deserialize, Serialize};

use crate::app::file_system::file::{LocalResource, LocalResourcePath};
use crate::app::transfer::session::{TransferProgress, TransferSession, TransferSessionStatus};
use crate::app::AppRequestBuilder;
use crate::errors::NetworkError;

use super::{CoreOperation, CoreOperationOutput};

/// This operation is used to access the local storage of device.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TransferOperation {
    CreateCloudSession(TransferSession),
    SendSession(TransferSession),
    AnswerSessionRequest {
        peer_id: String,
        session: Option<TransferSession>,
        session_id: u64
    },
    CancelSession(Option<String>, u64),
    FindPublicSession {
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
    CreateCloudSession(TransferSession),
    TransferResourceProgressUpdate(TransferProgress),
    TransferCompleted(TransferSessionStatus),
    ThumbnailFullFilled {
        local_resource_path: LocalResourcePath,
        resource_id: u64,
        session_id: u64
    },
    TransferCanceled,
    FindPublicSession(Option<TransferSession>),
    PublicTransferSessionUpdated((Vec<LocalResource>, Vec<TransferProgress>)),
    UnauthenticatedToSubscribeSession,
    SubscribeSessionEnded
}

impl Operation for TransferOperation {
    type Output = TransferOperationOutput;
}

impl TransferOperation {
    pub fn send_session(session: TransferSession) -> AppRequestBuilder<impl Future<Output = Result<(), NetworkError>>> {
        Command::request_from_shell(CoreOperation::Transfer(TransferOperation::SendSession(session))).map(|it| match it {
            CoreOperationOutput::Void => Ok(()),
            CoreOperationOutput::ConnectionError(error) => Err(error),
            _ => panic!("Mismatch in response type, expected Void, got {it:?}")
        })
    }

    pub fn cancel_session(
        peer_id: Option<String>,
        session_id: u64
    ) -> AppRequestBuilder<impl Future<Output = Result<(), NetworkError>>> {
        Command::request_from_shell(CoreOperation::Transfer(TransferOperation::CancelSession(peer_id, session_id))).map(
            |it| match it {
                CoreOperationOutput::Transfer(TransferOperationOutput::TransferCanceled) => Ok(()),
                CoreOperationOutput::ConnectionError(error) => Err(error),
                _ => panic!("Mismatch in response type, expected Void, got {it:?}")
            }
        )
    }

    pub fn create_cloud_session(
        session: TransferSession
    ) -> AppRequestBuilder<impl Future<Output = Result<TransferSession, NetworkError>>> {
        Command::request_from_shell(CoreOperation::Transfer(TransferOperation::CreateCloudSession(session))).map(|it| match it {
            CoreOperationOutput::Transfer(TransferOperationOutput::CreateCloudSession(session)) => Ok(session),
            CoreOperationOutput::ConnectionError(error) => Err(error),
            _ => panic!("Mismatch in response type, expected Void, got {it:?}")
        })
    }

    pub fn find_transfer_session(
        alias: String
    ) -> AppRequestBuilder<impl Future<Output = Result<Option<TransferSession>, NetworkError>>> {
        Command::request_from_shell(CoreOperation::Transfer(TransferOperation::FindPublicSession { alias })).map(|it| match it {
            CoreOperationOutput::Transfer(TransferOperationOutput::FindPublicSession(session)) => Ok(session),
            CoreOperationOutput::ConnectionError(error) => Err(error),
            _ => panic!("Mismatch in response type, expected Void, got {it:?}")
        })
    }
}
