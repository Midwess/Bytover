use std::sync::Arc;

use tokio::sync::OnceCell;

use crate::app::operations::transfer::{TransferOperation, TransferOperationOutput};
use crate::app::operations::CoreOperationOutput;
use crate::network::webrtc::connection::ConnectionWebRtcErrors;
use crate::network::webrtc::web_rtc::WebRtc;
use crate::ShellRuntime;

pub struct TransferNative {
    pub web_rtc: Arc<WebRtc>,
    pub shell_runtime: OnceCell<Arc<dyn ShellRuntime>>
}

impl TransferNative {
    pub fn update_shell_runtime(&self, shell_runtime: &Arc<dyn ShellRuntime>) {
        if self.shell_runtime.get().is_none() {
            let _ = self.shell_runtime.set(shell_runtime.clone());
        }
    }

    pub fn shell_runtime(&self) -> Arc<dyn ShellRuntime> {
        self.shell_runtime.get().unwrap().clone()
    }

    pub async fn handle(&self, request_id: u32, effect: TransferOperation) -> CoreOperationOutput {
        match effect {
            TransferOperation::SendSession(session) => {
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
                    Ok(_) => CoreOperationOutput::Void,
                    Err(e) => CoreOperationOutput::ConnectionError(e.into())
                }
            }
            TransferOperation::AnswerSessionRequest(peer_id, resources, session_id, peer_request_id, response) => {
                let Some(connection) = self.web_rtc.get_connection(peer_id).await.ok().and_then(|connection| connection.upgrade())
                else {
                    return CoreOperationOutput::ConnectionError(ConnectionWebRtcErrors::ConnectionNotFound.into());
                };

                let result = connection
                    .answer_session_request(request_id, resources, session_id, peer_request_id, response)
                    .await;

                log::info!(target: "transfer", "Answered session request: {:?}", result);

                match result {
                    Ok(_) => CoreOperationOutput::Transfer(TransferOperationOutput::TransferCompleted),
                    Err(error) => CoreOperationOutput::ConnectionError(error.into())
                }
            }
        }
    }
}
