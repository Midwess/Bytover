use std::future::Future;

use crux_core::capability::Operation;
use crux_core::Command;
use schema::devlog::bitbridge::peer_message_body::Response;
use serde::{Deserialize, Serialize};
use uniffi::Enum;

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
        thumbnail_dir: String,
        peer_id: u128,
        session: TransferSession,
        peer_request_id: String,
        response: Response
    },
    CancelSession(u128, u64)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Enum)]
pub enum TransferOperationOutput {
    CreateCloudSession(TransferSession),
    TransferResourceProgressUpdate(TransferProgress),
    TransferCompleted(TransferSessionStatus),
    TransferCanceled
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

    pub fn cancel_session(peer_id: u128, session_id: u64) -> AppRequestBuilder<impl Future<Output = Result<(), NetworkError>>> {
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
}
