use crate::app::operations::transfer::{TransferOperation, TransferOperationOutput};
use crate::app::operations::CoreOperationOutput;
use crate::entities::target::TransferTarget;
use crate::entities::transfer_session::{TransferSession, TransferType};
use crate::errors::CoreError;
use crate::protocol::public_cloud::cloud_service::CloudService;
use crate::protocol::rpc::app_server::AppServer;
use crate::protocol::rpc::cloud_server::CloudServer;
use crate::protocol::webrtc::webrtc::WebRtc;
use crate::shell::api::CoreRequest;
use core_services::utils::cancellation::CancellationToken;
use core_services::utils::maybe::MaybeSend;
use std::sync::Arc;

#[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
#[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
pub trait TransferNative<T>: Send + Sync
where
    T: 'static,
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

    fn app_server(&self) -> &AppServer<T>;

    async fn handle(&self, request: CoreRequest, effect: TransferOperation) -> Result<CoreOperationOutput, CoreError> {
        match effect {
            TransferOperation::CreateCloudSession(session) => {
                let session = self.cloud_service().create_public_session(session).await?;
                Ok(CoreOperationOutput::TransferSession(session))
            }
            TransferOperation::SendSession(session) => {
                let status = self.cloud_service().send_session(session, request).await?;
                Ok(TransferOperationOutput::TransferCompleted(status).into())
            }
            TransferOperation::CancelSession(peer_id, session_id) => {
                log::info!(target: "executor", "Cancelling session: {session_id:?}");

                if self.cloud_service().cancel(session_id).await {
                    return Ok(CoreOperationOutput::None);
                }

                let Some(peer_id) = peer_id else {
                    return Ok(CoreOperationOutput::None);
                };

                self.web_rtc().cancel_session(peer_id, session_id).await?;
                Ok(CoreOperationOutput::None)
            }
            TransferOperation::FindPublicSession { alias } => {
                // Try P2P session first
                if let Ok(Some(p2p_session)) = self.app_server().find_p2p_session_by_alias(alias.clone()).await {
                    let Some(user) = self.app_server().find_user(p2p_session.owner_user_id).await? else {
                        return Err(CoreError::BadRequest("Not found session".to_owned()));
                    };

                    // Create P2P transfer session
                    let transfer_session = TransferSession {
                        order_id: p2p_session.session_id,
                        resources: vec![],
                        progress: vec![],
                        session_resource: None,
                        transfer_type: TransferType::Receive,
                        target: TransferTarget::P2P {
                            from_peer: None,
                            scope: crate::entities::finding_scope::FindingScope::new(&p2p_session.signalling_room_id),
                            connection_state: crate::entities::target::P2PConnectionState::NotConnected
                        },
                        access_url: p2p_session.access_url.clone(),
                        alias: alias.clone(),
                        from_user: user,
                        description: p2p_session.description.clone(),
                        password: None,
                        is_required_password: false,
                        connection_error: None,
                        cancellation_token: CancellationToken::new()
                    };

                    return Ok(Some(transfer_session).into());
                }

                // Fallback to public session search
                let response = self.cloud_server().find_public_session(alias).await?;
                let is_required_password = response.is_required_password;
                let access_url = response.access_url;
                let Some(session_key) = response.session else {
                    return Ok(CoreOperationOutput::None);
                };

                let Some(user) = self.app_server().find_user(session_key.user_id).await? else {
                    return Err(CoreError::BadRequest(format!("Not found owner {} for this session", session_key.user_id)));
                };

                let transfer_session = TransferSession::from_public_overview(
                    session_key.order_id,
                    user,
                    access_url.clone(),
                    response.alias.clone(),
                    is_required_password
                );

                Ok(Some(transfer_session).into())
            }
            TransferOperation::SubscribeToPublicSessionTransferProgress {
                password,
                session_owner_user_id,
                session_order_id
            } => {
                self.cloud_service()
                    .fetch_public_session(request, session_order_id, session_owner_user_id, password)
                    .await?;
                Ok(TransferOperationOutput::SubscribeSessionEnded.into())
            }
        }
    }
}
