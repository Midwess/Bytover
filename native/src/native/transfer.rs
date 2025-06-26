use std::sync::Arc;

use tokio::sync::OnceCell;

use crate::network::cloud::cloud_service::CloudService;
use crate::network::webrtc::connection::ConnectionWebRtcErrors;
use crate::network::webrtc::web_rtc::WebRtc;
use crate::ShellRuntime;
use shared::app::operations::transfer::{TransferOperation, TransferOperationOutput};
use shared::app::operations::CoreOperationOutput;
use shared::errors::NetworkError;

pub struct TransferNative {
    pub web_rtc: Arc<WebRtc>,
    pub cloud_service: CloudService,
    pub shell_runtime: OnceCell<Arc<dyn ShellRuntime>>
}

impl TransferNative {
    pub fn update_shell_runtime(&self, shell_runtime: &Arc<dyn ShellRuntime>) {
        if self.shell_runtime.get().is_none() {
            let _ = self.shell_runtime.set(shell_runtime.clone());
            self.cloud_service.init(shell_runtime.clone());
        }
    }

    pub fn shell_runtime(&self) -> Arc<dyn ShellRuntime> {
        self.shell_runtime.get().unwrap().clone()
    }

    pub async fn handle(&self, request_id: u32, effect: TransferOperation) -> CoreOperationOutput {
        match effect {
            TransferOperation::CreateCloudSession(session) => match self.cloud_service.create_public_session(session).await {
                Ok(session) => CoreOperationOutput::Transfer(TransferOperationOutput::CreateCloudSession(session)),
                Err(e) => {
                    log::error!("Create public session error: {:?}", e);
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

                let Some(connection) = self
                    .web_rtc
                    .get_connection(session.peer_id().unwrap_or_default())
                    .await
                    .ok()
                    .and_then(|connection| connection.upgrade())
                else {
                    return CoreOperationOutput::ConnectionError(ConnectionWebRtcErrors::ConnectionNotFound.into());
                };

                match connection.send_session(session, request_id).await {
                    Ok(status) => CoreOperationOutput::Transfer(TransferOperationOutput::TransferCompleted(status)),
                    Err(e) => CoreOperationOutput::ConnectionError(e.into())
                }
            }
            TransferOperation::AnswerSessionRequest {
                peer_id,
                session,
                peer_request_id,
                response,
                ..
            } => {
                let Some(connection) = self.web_rtc.get_connection(peer_id).await.ok().and_then(|connection| connection.upgrade())
                else {
                    return CoreOperationOutput::ConnectionError(ConnectionWebRtcErrors::ConnectionNotFound.into());
                };

                let result = connection.answer_session_request(request_id, session, peer_request_id, response).await;

                log::info!(target: "transfer", "Answered session request: {:?}", result);

                match result {
                    Ok(status) => CoreOperationOutput::Transfer(TransferOperationOutput::TransferCompleted(status)),
                    Err(error) => CoreOperationOutput::ConnectionError(error.into())
                }
            }
            TransferOperation::CancelSession(peer_id, session_id) => {
                log::info!(target: "native", "Cancelling session: {:?}", session_id);

                if self.cloud_service.cancel(session_id).await {
                    return CoreOperationOutput::Transfer(TransferOperationOutput::TransferCanceled);
                }

                let Some(peer_id) = peer_id else {
                    log::error!(target: "native", "Peer ID is not provided");
                    return CoreOperationOutput::ConnectionError(ConnectionWebRtcErrors::ConnectionNotFound.into());
                };

                let Some(connection) = self.web_rtc.get_connection(peer_id).await.ok().and_then(|connection| connection.upgrade())
                else {
                    return CoreOperationOutput::ConnectionError(ConnectionWebRtcErrors::ConnectionNotFound.into());
                };

                connection.stop_session(session_id).await;

                CoreOperationOutput::Transfer(TransferOperationOutput::TransferCanceled)
            }
        }
    }
}
