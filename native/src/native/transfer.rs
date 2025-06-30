use std::sync::Arc;

use tokio::sync::OnceCell;

use crate::network::cloud::cloud_service::CloudService;
use crate::ShellRuntime;
use shared::app::operations::transfer::{TransferOperation, TransferOperationOutput};
use shared::app::operations::CoreOperationOutput;
use shared::core_transfer_protocol::webrtc::webrtc::WebRtc;
use shared::errors::NetworkError;

pub struct TransferNative {
    pub web_rtc: Arc<WebRtc>,
    pub cloud_service: CloudService,
    pub shell_runtime: Arc<dyn ShellRuntime>
}

impl TransferNative {
    pub async fn handle(&self, request_id: u32, effect: TransferOperation) -> CoreOperationOutput {
        match effect {
            TransferOperation::CreateCloudSession(session) => match self.cloud_service.create_public_session(session).await {
                Ok(session) => CoreOperationOutput::Transfer(TransferOperationOutput::CreateCloudSession(session)),
                Err(e) => {
                    log::error!("Create public session error: {e:?}");
                    CoreOperationOutput::ConnectionError(NetworkError::InternalServerError(e.to_string()))
                }
            },
            TransferOperation::SendSession(session) => {
                if session.target.is_public() {
                    return match self.cloud_service.send_session(session, request_id).await {
                        Ok(it) => CoreOperationOutput::Transfer(TransferOperationOutput::TransferCompleted(it)),
                        Err(e) => CoreOperationOutput::ConnectionError(e.into())
                    }
                }

                match self.web_rtc.send_session(request_id, session).await {
                    Ok(status) => CoreOperationOutput::Transfer(TransferOperationOutput::TransferCompleted(status)),
                    Err(e) => CoreOperationOutput::ConnectionError(e.into())
                }
            }
            TransferOperation::AnswerSessionRequest {
                peer_id,
                session,
                session_id
            } => {
                let result = self.web_rtc.answer_session(
                    request_id,
                    peer_id,
                    session,
                    session_id
                ).await;

                log::info!(target: "transfer", "Answered session request: {result:?}");

                match result {
                    Ok(status) => CoreOperationOutput::Transfer(TransferOperationOutput::TransferCompleted(status)),
                    Err(error) => CoreOperationOutput::ConnectionError(error.into())
                }
            }
            TransferOperation::CancelSession(peer_id, session_id) => {
                log::info!(target: "native", "Cancelling session: {session_id:?}");

                if self.cloud_service.cancel(session_id).await {
                    return CoreOperationOutput::Transfer(TransferOperationOutput::TransferCanceled);
                }
                
                if peer_id.is_none() {
                    return CoreOperationOutput::Transfer(TransferOperationOutput::TransferCanceled);
                }

                let _ = self.web_rtc.cancel_session(peer_id.unwrap(), session_id).await;

                CoreOperationOutput::Transfer(TransferOperationOutput::TransferCanceled)
            }
        }
    }
}
