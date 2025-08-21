use crate::app::operations::transfer::{TransferOperation, TransferOperationOutput};
use crate::app::operations::CoreOperationOutput;
use crate::app::transfer::session::TransferSession;
use crate::core_transfer_protocol::public_cloud::cloud_service::CloudService;
use crate::core_transfer_protocol::webrtc::webrtc::WebRtc;
use crate::errors::NetworkError;
use crate::rpc::auth_server::AuthServer;
use crate::rpc::cloud_server::CloudServer;
use core_services::utils::maybe::MaybeSend;
use std::sync::Arc;

#[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
#[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
pub trait TransferNative<T>: Send + Sync
where
    T: Clone,
    T: MaybeSend + Sync,
    T: tonic::client::GrpcService<tonic::body::Body>,
    T::Error: Into<tonic::codegen::StdError>,
    T::ResponseBody: http_body::Body<Data = bytes::Bytes> + Send + 'static,
    <T::ResponseBody as http_body::Body>::Error: Into<tonic::codegen::StdError> + Send,
    T::Future: MaybeSend
{
    fn web_rtc(&self) -> &Arc<WebRtc>;

    fn cloud_service(&self) -> &CloudService<T>;

    fn cloud_server(&self) -> &CloudServer<T>;

    fn auth_server(&self) -> &AuthServer<T>;

    async fn handle(&self, request_id: u32, effect: TransferOperation) -> CoreOperationOutput {
        match effect {
            TransferOperation::CreateCloudSession(session) => match self.cloud_service().create_public_session(session).await {
                Ok(session) => CoreOperationOutput::Transfer(TransferOperationOutput::CreateCloudSession(session)),
                Err(e) => {
                    log::error!("Create public session error: {e:?}");
                    CoreOperationOutput::ConnectionError(NetworkError::InternalServerError(e.to_string()))
                }
            },
            TransferOperation::SendSession(session) => {
                if session.target.is_public() {
                    return match self.cloud_service().send_session(session, request_id).await {
                        Ok(it) => CoreOperationOutput::Transfer(TransferOperationOutput::TransferCompleted(it)),
                        Err(e) => CoreOperationOutput::ConnectionError(e.into())
                    }
                }

                match self.web_rtc().send_session(request_id, session).await {
                    Ok(status) => CoreOperationOutput::Transfer(TransferOperationOutput::TransferCompleted(status)),
                    Err(e) => CoreOperationOutput::ConnectionError(e.into())
                }
            }
            TransferOperation::AnswerSessionRequest {
                peer_id,
                session,
                session_id
            } => {
                let result = self.web_rtc().answer_session(request_id, peer_id, session, session_id).await;

                log::info!(target: "transfer", "Answered session request: {result:?}");

                match result {
                    Ok(status) => CoreOperationOutput::Transfer(TransferOperationOutput::TransferCompleted(status)),
                    Err(error) => CoreOperationOutput::ConnectionError(error.into())
                }
            }
            TransferOperation::CancelSession(peer_id, session_id) => {
                log::info!(target: "executor", "Cancelling session: {session_id:?}");

                if self.cloud_service().cancel(session_id).await {
                    return CoreOperationOutput::Transfer(TransferOperationOutput::TransferCanceled);
                }

                if peer_id.is_none() {
                    return CoreOperationOutput::Transfer(TransferOperationOutput::TransferCanceled);
                }

                let _ = self.web_rtc().cancel_session(peer_id.unwrap(), session_id).await;

                CoreOperationOutput::Transfer(TransferOperationOutput::TransferCanceled)
            }
            TransferOperation::FindPublicSession { alias } => {
                let (is_required_password, session_order_id, user_id, access_url) =
                    match self.cloud_server().find_public_session(alias).await {
                        Ok(response) => {
                            let Some(session_key) = response.session else {
                                return CoreOperationOutput::Transfer(TransferOperationOutput::FindPublicSession(None));
                            };

                            (
                                response.is_required_password,
                                session_key.order_id,
                                session_key.user_id,
                                response.access_url
                            )
                        }
                        Err(e) => return CoreOperationOutput::ConnectionError(e.into())
                    };

                let user = match self.auth_server().find_user(user_id).await {
                    Ok(Some(user)) => user,
                    Ok(None) => {
                        return CoreOperationOutput::ConnectionError(NetworkError::BadRequest("Not found session".to_owned()));
                    }
                    Err(e) => return CoreOperationOutput::ConnectionError(e.into())
                };

                let transfer_session = TransferSession::from_public_overview(session_order_id, user, access_url, is_required_password);

                CoreOperationOutput::Transfer(TransferOperationOutput::FindPublicSession(Some(transfer_session)))
            }
            TransferOperation::SubscribeToPublicSessionTransferProgress {
                password,
                session_owner_user_id,
                session_order_id
            } => {
                if let Err(e) = self
                    .cloud_service()
                    .fetch_public_session(request_id, session_order_id, session_owner_user_id, password)
                    .await
                {
                    log::error!("Fetch public session error: {e:?}");
                    return CoreOperationOutput::ConnectionError(e.into());
                }

                CoreOperationOutput::Transfer(TransferOperationOutput::SubscribeSessionEnded)
            }
        }
    }
}
