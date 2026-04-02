use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use core_services::utils::cancellation::{FutureExtension, TaskErrors};
use core_services::utils::yield_container::{YieldContainer, YieldError};
use futures::channel::mpsc;
use futures::channel::mpsc::unbounded;
use futures::select_biased;
use futures::stream::StreamExt;
use futures_util::lock::Mutex;
use futures_util::{FutureExt, SinkExt};
use once_cell::sync::OnceCell;
use schema::devlog::bitbridge::peer_message_body::{Request, Response};
use schema::devlog::bitbridge::{P2pCancelSessionRequest, ResourceTypeMessage, VoidResponseMessage};
use shared::app::operations::p2p::P2POperationOutput;
use shared::app::operations::transfer::TransferOperationOutput;
use shared::app::operations::CoreOperationOutput;
use shared::entities::local_resource::{LocalResource, LocalResourcePath, ResourceType};
use shared::entities::peer::Peer as PeerEntity;
use shared::entities::transfer_session::TransferProgress;
use shared::protocol::webrtc::message_channel::DirectMessageChannel;
use shared::protocol::webrtc::packet::WebRtcPacket;
use shared::protocol::webrtc::transfer::{TransferDelimiterShema, TransfersContext};
use shared::repository::local_resource::LocalResourceRepository;
use shared::repository::transfer_session::TransferSessionRepository;
use shared::shell::api::CoreRequest;

use crate::webrtc::ice::{IceAgent, IceError};
use crate::webrtc::signaling::{SignalingClient, SignalingError};
use crate::webrtc::web::channel_ids::*;
use crate::webrtc::web::{RtcConnectionWrapper, RtcDataChannelWrapper, WebError, WebRtcApi};

pub struct WebRtcClient {
    transfers_context: TransfersContext,
    me: OnceCell<PeerEntity>,
    peer: OnceCell<PeerEntity>,
    core_request: OnceCell<CoreRequest>,
    resource_repo: Arc<dyn LocalResourceRepository>,
    transfer_session_repo: Arc<dyn TransferSessionRepository>,
    inbound_msg_receiver: YieldContainer<mpsc::UnboundedReceiver<Vec<u8>>>,
    inbound_data_receiver: YieldContainer<mpsc::UnboundedReceiver<Vec<u8>>>,

    msg_channel: OnceCell<DirectMessageChannel>,
    unordered_msg_channel: OnceCell<DirectMessageChannel>,
    prefix_channels: Mutex<HashMap<u16, mpsc::Sender<(u64, Vec<u8>)>>>,
    connection: OnceCell<Arc<RtcConnectionWrapper>>,
    reliable_channel: OnceCell<Arc<RtcDataChannelWrapper>>,
}

fn spawn_outbound_sender(channel: Arc<RtcDataChannelWrapper>, mut rx: mpsc::Receiver<Vec<u8>>) {
    wasm_bindgen_futures::spawn_local(async move {
        while let Some(data) = rx.next().await {
            let arr = js_sys::Uint8Array::from(&data[..]);
            let _ = channel.send_with_array_buffer_view(&arr);
        }
    });
}

impl WebRtcClient {
    pub async fn connect(
        me: PeerEntity,
        signaling: SignalingClient,
        ice_agent: IceAgent,
        peer_id: &str,
        resource_repo: Arc<dyn LocalResourceRepository>,
        transfer_session_repo: Arc<dyn TransferSessionRepository>
    ) -> Result<Arc<Self>, WebRtcClientError> {
        log::info!("WebRtcClient connecting to peer {}", peer_id);

        let ice_config = signaling.fetch_relay_config(peer_id).await.unwrap_or_else(|e| {
            log::warn!("Failed to fetch relay config: {:?}", e);
            schema::devlog::rpc_signalling::server::IceConfig {
                urls: vec!["stun:stun.l.google.com:19302".to_string()],
                ..Default::default()
            }
        });

        let api = WebRtcApi::new(ice_config);
        let connection = api.create_peer_connection()?;

        let (msg_inbound_tx, msg_inbound_rx) = unbounded();
        let (data_inbound_tx, data_inbound_rx) = unbounded();
        let (ordered_out_tx, ordered_out_rx) = mpsc::channel(16);
        let (unordered_out_tx, unordered_out_rx) = mpsc::channel(16);

        let reliable_channel = api.create_unordered_channel(connection.clone(), RELIABLE_DATA_CHANNEL_ID)?;
        let unordered_channel = api.create_unordered_channel(connection.clone(), UNORDERED_MSG_CHANNEL_ID)?;
        let ordered_channel = api.create_ordered_channel(connection.clone(), ORDERED_MSG_CHANNEL_ID)?;

        api.setup_channel_handlers(reliable_channel.clone(), data_inbound_tx.clone())?;
        api.setup_channel_handlers(unordered_channel.clone(), msg_inbound_tx.clone())?;
        api.setup_channel_handlers(ordered_channel.clone(), msg_inbound_tx)?;

        api.create_offer_and_set_local(&connection).await?;

        ice_agent.wait_for_gathering_complete(&connection).await?;

        let local_sdp = connection
            .local_description()
            .ok_or_else(|| WebRtcClientError::Connection("No local description".to_string()))?
            .sdp();

        log::info!("ICE gathering complete, local SDP ready, sending to signaling {}", local_sdp);

        let answer_sdp = signaling.send_offer(peer_id, &local_sdp).await?;

        log::info!("Got answer from remote peer {answer_sdp:?}");
        api.set_remote_description(&connection, &answer_sdp).await?;

        let client = Arc::new(WebRtcClient {
            transfers_context: TransfersContext::new(),
            me: OnceCell::new(),
            peer: OnceCell::new(),
            core_request: OnceCell::new(),
            resource_repo,
            transfer_session_repo,
            inbound_msg_receiver: YieldContainer::new(msg_inbound_rx),
            inbound_data_receiver: YieldContainer::new(data_inbound_rx),
            msg_channel: OnceCell::new(),
            unordered_msg_channel: OnceCell::new(),
            prefix_channels: Mutex::new(HashMap::new()),
            connection: OnceCell::new(),
            reliable_channel: OnceCell::new(),
        });

        let _ = client.connection.set(connection);
        let _ = client.reliable_channel.set(reliable_channel);

        let _ = client.msg_channel.set(DirectMessageChannel::new(ordered_out_tx));
        let _ = client.unordered_msg_channel.set(DirectMessageChannel::new(unordered_out_tx));

        spawn_outbound_sender(ordered_channel.clone(), ordered_out_rx);
        spawn_outbound_sender(unordered_channel.clone(), unordered_out_rx);

        api.wait_for_channel_open(ordered_channel).await?;

        log::info!("WebRtcClient connection established, performing introduce handshake...");

        let mut msg_rx = client.inbound_msg_receiver.retrieve().await?;

        let client_clone = client.clone();
        let me_clone = me.clone();
        wasm_bindgen_futures::spawn_local(async move {
            if let Err(e) = client_clone.introduce(&me_clone).await {
                log::error!("Failed to introduce on connect: {:?}", e);
            }
        });

        while let Some(packet) = msg_rx.next().await {
            if let Some(msg_channel) = client.msg_channel.get() {
                match msg_channel.receive_packet(packet).await {
                    Ok(Some(msg)) => {
                        if let Some(request) = msg.request {
                            client.process_message_packet(msg.request_id, request).await;
                        }
                    }
                    Ok(None) => {}
                    Err(e) => {
                        log::warn!("Failed to decode packet in initial connect loop: {:?}", e);
                    }
                }
            }
            if client.peer.get().is_some() {
                break;
            }
        }

        drop(msg_rx);

        log::info!("WebRtcClient introduce complete");
        Ok(client)
    }

    pub async fn run(self: Arc<Self>) -> Result<(), WebRtcClientError> {
        log::info!("WebRtcClient run loop starting");

        let msg_future = self.message_loop();
        let data_future = self.data_receiving_loop();

        futures::pin_mut!(msg_future, data_future);
        select_biased! {
            r = msg_future.fuse() => r?,
            r = data_future.fuse() => r?,
        }

        log::info!("WebRtcClient run loop terminated");
        Ok(())
    }

    pub fn peer_id(&self) -> Option<String> {
        self.peer.get().map(|p| p.id.clone())
    }

    pub fn peer_entity(&self) -> Option<PeerEntity> {
        self.peer.get().cloned()
    }

    pub fn set_peer(&self, peer: PeerEntity) -> Result<(), PeerEntity> {
        self.peer.set(peer)
    }

    pub async fn introduce(&self, current_user: &PeerEntity) -> Result<(), WebRtcClientError> {
        log::info!("Starting introduce handshake");
        let _ = self.me.set(current_user.clone());

        let introduce_request = schema::devlog::bitbridge::IntroduceRequestMessage {
            mine: schema::devlog::bitbridge::PeerMessage {
                peer_id: current_user.id.clone(),
                name: current_user.name.clone(),
                avatar_url: current_user.avatar_url.clone(),
                device: current_user.device.clone().into(),
                email: current_user.email.clone()
            }
        };

        let msg_channel = self
            .msg_channel
            .get()
            .ok_or_else(|| WebRtcClientError::Connection("No message channel".to_string()))?;

        let response = msg_channel
            .send(Request::IntroduceRequest(introduce_request), None)
            .await
            .map_err(|e| WebRtcClientError::Message(e.to_string()))?;

        match response {
            Response::IntroduceResponse(resp) => {
                let peer = PeerEntity {
                    id: resp.peer.peer_id.clone(),
                    name: resp.peer.name.clone(),
                    avatar_url: resp.peer.avatar_url.clone(),
                    device: resp.peer.device.clone().into(),
                    email: resp.peer.email.clone(),
                    user_id: None,
                    signalling_id: None
                };
                let _ = self.set_peer(peer);
                log::info!("Introduce handshake completed");
                Ok(())
            }
            _ => Err(WebRtcClientError::Message("Unexpected response type".to_string()))
        }
    }

    pub async fn request_session_detail(
        &self,
        core_request: CoreRequest,
        order_id: u64,
        password: Option<String>
    ) -> Result<(), WebRtcClientError> {
        use schema::devlog::bitbridge::view_session_detail_response::Result as ResponseResult;
        use schema::devlog::bitbridge::PeerErrorsMessage;
        use core_services::utils::cancellation::CancellationToken;

        let request = schema::devlog::bitbridge::ViewSessionDetailRequest { order_id, password };

        let msg_channel = self
            .msg_channel
            .get()
            .ok_or_else(|| WebRtcClientError::Connection("No message channel".to_string()))?;

        let timeout_token = CancellationToken::timeout(Duration::from_secs(60));

        let response = msg_channel
            .send(Request::ViewSessionRequest(request), None)
            .with_cancel(&timeout_token)
            .await
            .map_err(|_| WebRtcClientError::Timeout)?
            .map_err(|e| WebRtcClientError::Message(e.to_string()))?;

        match response {
            Response::ViewSessionResponse(resp) => match resp.result {
                Some(ResponseResult::Session(session)) => {
                    core_request
                        .response(CoreOperationOutput::Transfer(TransferOperationOutput::SessionDetailReceived(session)))
                        .await;
                }
                Some(ResponseResult::Error(error_type)) => {
                    let error_msg = PeerErrorsMessage::try_from(error_type).unwrap_or(PeerErrorsMessage::InvalidRequest);
                    core_request
                        .response(CoreOperationOutput::Error(shared::errors::CoreError::PeerRequestError(error_msg)))
                        .await;
                    return Err(WebRtcClientError::Peer(error_msg.to_string()));
                }
                _ => return Err(WebRtcClientError::Message("Unexpected response".to_string()))
            },
            _ => return Err(WebRtcClientError::Message("Unexpected response type".to_string()))
        }

        Ok(())
    }

    pub async fn request_resource_download(
        &self,
        core_request: CoreRequest,
        session_order_id: u64,
        resource: LocalResource,
        mut progress: TransferProgress
    ) -> Result<TransferProgress, WebRtcClientError> {
        use schema::devlog::bitbridge::DownloadResourceRequest;
        use std::sync::atomic::{AtomicU16, Ordering};

        static TRANSFER_ID_COUNTER: AtomicU16 = AtomicU16::new(1);

        let resource_order_id = resource.order_id;
        let transfer_id = TRANSFER_ID_COUNTER.fetch_add(1, Ordering::SeqCst);

        let request = DownloadResourceRequest {
            session_order_id,
            resource_order_id,
            transfer_id: transfer_id as u32
        };

        let (tx, mut rx) = mpsc::channel::<(u64, Vec<u8>)>(10);
        self.prefix_channels.lock().await.insert(transfer_id, tx);

        let resource_token = self
            .transfers_context
            .get_or_create_resource_token(session_order_id, resource_order_id)
            .await;

        progress.update_progress(1);
        core_request
            .response(TransferOperationOutput::TransferResourceProgressUpdate(progress.clone()))
            .await;

        let msg_channel = self
            .msg_channel
            .get()
            .ok_or_else(|| WebRtcClientError::Connection("No message channel".to_string()))?;

        msg_channel
            .notify(Request::DownloadResourceRequest(request))
            .await
            .map_err(|e| WebRtcClientError::Message(e.to_string()))?;

        let start_delimiter = loop {
            match rx.next().with_cancel(&resource_token).await? {
                Some((_, packet)) => {
                    if let Ok(v) = TransferDelimiterShema::from_start_packet(&packet, session_order_id) {
                        break v;
                    }
                }
                None => {
                    self.prefix_channels.lock().await.remove(&transfer_id);
                    return Err(WebRtcClientError::Transfer("Channel closed before start delimiter".into()));
                }
            }
        };

        let compressed = start_delimiter.compressed();
        let mut writer = self
            .resource_repo
            .write(resource.path.clone(), compressed)
            .await
            .map_err(|e| WebRtcClientError::Transfer(format!("Failed to create writer: {:?}", e)))?;

        let mut expected_size: Option<u64> = None;
        loop {
            match rx.next().with_cancel(&resource_token).await? {
                Some((offset, packet)) => {
                    if let Ok(delimiter) = TransferDelimiterShema::from_bytes(&packet) {
                        if matches!(delimiter, TransferDelimiterShema::End { .. }) && delimiter.session_id() == Some(session_order_id) {
                            expected_size = delimiter.total_size();

                            if let Some(target) = expected_size {
                                if progress.total_bytes() >= target {
                                    progress.success();
                                    core_request
                                        .response(TransferOperationOutput::TransferResourceProgressUpdate(progress.clone()))
                                        .await;
                                    break;
                                }
                            } else {
                                progress.success();
                                core_request
                                    .response(TransferOperationOutput::TransferResourceProgressUpdate(progress.clone()))
                                    .await;
                                break;
                            }
                            continue;
                        }
                    }

                    // Stop tracking transfer if context is cancelled
                    if resource_token.is_cancelled() {
                        break;
                    }

                    let bytes = Bytes::from(packet.to_vec());
                    if let Some(written) = writer.d_write_at(bytes, offset).await.map_err(|e| WebRtcClientError::Transfer(e.to_string()))? {
                        progress.update_progress(written as u64);
                        core_request
                            .response_throttle(TransferOperationOutput::TransferResourceProgressUpdate(progress.clone()))
                            .await;

                        if let Some(target) = expected_size {
                            if progress.total_bytes() >= target {
                                progress.success();
                                core_request
                                    .response(TransferOperationOutput::TransferResourceProgressUpdate(progress.clone()))
                                    .await;
                                break;
                            }
                        }
                    }
                }
                None => {
                    self.prefix_channels.lock().await.remove(&transfer_id);
                    return Err(WebRtcClientError::Transfer("Channel closed before completion".into()));
                }
            }
        }

        self.prefix_channels.lock().await.remove(&transfer_id);
        Ok(progress)
    }

    pub async fn download_all_resources(
        &self,
        core_request: CoreRequest,
        session_order_id: u64,
        session_resource: LocalResource,
        resources: Vec<LocalResource>
    ) -> Result<TransferProgress, WebRtcClientError> {
        use shared::entities::transfer_session::TransferType;

        let token = self
            .transfers_context
            .get_or_create_resource_token(session_order_id, session_resource.order_id)
            .await;
        let zip_path = session_resource.path.clone();

        if let Err(e) = self.transfer_session_repo.start_download_session(zip_path.clone()).await {
            self.cancel_resource_transfer(session_order_id, session_resource.order_id).await;
            return Err(WebRtcClientError::Transfer(format!("Failed to start download session: {:?}", e)));
        }

        let mut download_failed = false;

        for resource in resources {
            let resource_id = resource.order_id;
            let progress = TransferProgress::new(resource_id, resource.size, TransferType::Receive);

            let result = self
                .request_resource_download(core_request.clone(), session_order_id, resource, progress)
                .with_cancel(&token)
                .await;

            match result {
                Ok(Ok(_)) => {}
                Err(_) => {
                    self.cancel_resource_transfer(session_order_id, resource_id).await;
                    download_failed = true;
                    break;
                }
                Ok(Err(e)) => {
                    log::error!("Failed to download resource {}: {:?}", resource_id, e);
                    self.cancel_resource_transfer(session_order_id, resource_id).await;
                    download_failed = true;
                    break;
                }
            }
        }

        if download_failed {
            let _ = self.transfer_session_repo.stop_download_session(zip_path).await;
            self.cancel_resource_transfer(session_order_id, session_resource.order_id).await;
            return Err(WebRtcClientError::Transfer("Download all failed".into()));
        }

        let mut session_progress = TransferProgress::new(session_resource.order_id, session_resource.size, TransferType::Receive);
        session_progress.update_progress(session_resource.size);
        core_request
            .response(TransferOperationOutput::TransferResourceProgressUpdate(session_progress.clone()))
            .await;

        if let Err(e) = self.transfer_session_repo.stop_download_session(zip_path).await {
            session_progress.fail(format!("Failed to save: {:?}", e));
            core_request
                .response(TransferOperationOutput::TransferResourceProgressUpdate(session_progress))
                .await;
            return Err(WebRtcClientError::Transfer("Failed to stop download session".into()));
        }

        session_progress.success();
        core_request
            .response(TransferOperationOutput::TransferResourceProgressUpdate(session_progress.clone()))
            .await;

        Ok(session_progress)
    }

    pub fn start_core_stream(&self, core_request: CoreRequest) {
        let _ = self.core_request.set(core_request);
    }

    pub fn core_request(&self) -> Option<&CoreRequest> {
        self.core_request.get()
    }

    pub async fn process_message_packet(&self, request_id: String, msg: Request) {
        match msg {
            Request::IntroduceRequest(introduce_msg) => {
                log::info!("Received introduce request from peer");
                let peer = PeerEntity {
                    id: introduce_msg.mine.peer_id.clone(),
                    name: introduce_msg.mine.name.clone(),
                    avatar_url: introduce_msg.mine.avatar_url.clone(),
                    device: introduce_msg.mine.device.clone().into(),
                    email: introduce_msg.mine.email.clone(),
                    user_id: None,
                    signalling_id: None
                };
                log::info!("Remote peer: {:?}", peer.id);
                let _ = self.set_peer(peer);

                if let Some(me) = self.me.get() {
                    let response = schema::devlog::bitbridge::IntroduceResponseMessage {
                        peer: schema::devlog::bitbridge::PeerMessage {
                            peer_id: me.id.clone(),
                            name: me.name.clone(),
                            avatar_url: me.avatar_url.clone(),
                            device: me.device.clone().into(),
                            email: me.email.clone()
                        }
                    };
                    if let Some(msg_channel) = self.msg_channel.get() {
                        let _ = msg_channel
                            .send_response(request_id, Response::IntroduceResponse(response))
                            .await;
                    }
                } else {
                    log::warn!("Cannot respond to IntroduceRequest: current user not set (call introduce() first)");
                }
            }
            Request::CancelRequest(request) => {
                log::info!("Received cancel request {:?}", request);
                if let Some(resource_id) = request.resource_id {
                    self.transfers_context.cancel_resource(request.session_id, resource_id).await;
                } else {
                    self.transfers_context.cancel_transfer(request.session_id).await;
                }
            }
            Request::ResourceNotification(notification) => {
                let session_order_id = notification.session_order_id;
                log::info!("Received resource notification for session order_id {} resource_id {:?}", session_order_id, notification.resource.as_ref().map(|it| it.order_id)); 
                if let Some(resource_proto) = notification.resource {
                    let mut resource = LocalResource {
                        order_id: resource_proto.order_id,
                        name: resource_proto.name,
                        size: resource_proto.size as u64,
                        path: LocalResourcePath::RelativePath {
                            path: format!("received/session_{}/resource_{}", session_order_id, resource_proto.order_id),
                            is_private: false
                        },
                        thumbnail_path: None,
                        r#type: (ResourceTypeMessage::try_from(resource_proto.r#type).unwrap_or_default())
                            .try_into()
                            .unwrap_or(ResourceType::File),
                        shelf_id: 0
                    };

                    if let Some(thumbnail_bytes) = resource_proto.thumbnail_png {
                        match self.resource_repo.save_thumbnail(thumbnail_bytes, resource.order_id).await {
                            Ok(thumbnail_path) => {
                                resource.thumbnail_path = Some(thumbnail_path);
                            }
                            Err(e) => {
                                log::warn!("Failed to save thumbnail: {:?}", e);
                            }
                        }
                    }

                    if let Some(core_request) = self.core_request() {
                        core_request
                            .response(CoreOperationOutput::P2P(P2POperationOutput::ReceivedResourceNotification {
                                session_order_id,
                                resource,
                                peer_id: self.peer.get().map(|p| p.id.clone()).unwrap_or_default()
                            }))
                            .await;
                    }
                }

                if let Some(msg_channel) = self.msg_channel.get() {
                    let _ = msg_channel
                        .send_response(request_id, Response::VoidResponse(VoidResponseMessage {}))
                        .await;
                }
            }
            _ => {
                log::debug!("Unhandled message request type");
            }
        }
    }

    pub async fn cancel_transfer(&self, session_id: u64) {
        self.transfers_context.cancel_transfer(session_id).await;
        if let Some(msg_channel) = self.msg_channel.get() {
            let cancel_msg = P2pCancelSessionRequest {
                session_id,
                resource_id: None
            };
            let _ = msg_channel.notify(Request::CancelRequest(cancel_msg)).await;
        }
    }

    pub async fn cancel_resource_transfer(&self, session_id: u64, resource_id: u64) {
        self.transfers_context.cancel_resource(session_id, resource_id).await;
        if let Some(msg_channel) = self.msg_channel.get() {
            let cancel_msg = P2pCancelSessionRequest {
                session_id,
                resource_id: Some(resource_id)
            };
            let _ = msg_channel.notify(Request::CancelRequest(cancel_msg)).await;
        }
    }

    pub async fn peer_disconnected(&self) {
        log::info!("Peer disconnected");
        self.transfers_context.cancel_all_transfers().await;
    }

    async fn message_loop(&self) -> Result<(), WebRtcClientError> {
        log::info!("Starting message loop");

        let mut msg_rx = self.inbound_msg_receiver.retrieve().await?;

        let msg_channel = self
            .msg_channel
            .get()
            .ok_or_else(|| WebRtcClientError::Connection("No message channel".to_string()))?;

        while let Some(packet) = msg_rx.next().await {
            match msg_channel.receive_packet(packet).await {
                Ok(Some(msg)) => {
                    log::debug!("message_loop: decoded request, request_id={}", msg.request_id);
                    if let Some(request) = msg.request {
                        let request_id = msg.request_id;
                        self.process_message_packet(request_id, request).await;
                    }
                }
                Ok(None) => {
                    log::info!("message_loop: decoded response (routed to pending send)");
                }
                Err(e) => {
                    log::warn!("message_loop: failed to decode packet: {:?}", e);
                }
            }
        }

        log::info!("Message channel closed, terminating message loop");
        Ok(())
    }

    async fn data_receiving_loop(&self) -> Result<(), WebRtcClientError> {
        log::info!("Starting data receiving loop");

        let mut inbound_data_receiver = self.inbound_data_receiver.retrieve().await?;

        while let Some(data) = inbound_data_receiver.next().await {
            let (prefix, offset, payload) = WebRtcPacket::deserialize(data);
            let mut channels = self.prefix_channels.lock().await;
            if let Some(tx) = channels.get_mut(&prefix) {
                if let Err(e) = tx.send((offset, payload)).await {
                    log::warn!("Prefix channel {} dropped: {:?}", prefix, e);
                    channels.remove(&prefix);
                }
            }
        }

        log::info!("Receiving loop stopped");
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum WebRtcClientError {
    #[error("Signaling error: {0}")]
    Signaling(String),

    #[error("Connection error: {0}")]
    Connection(String),

    #[error("Transfer error: {0}")]
    Transfer(String),

    #[error("Message error: {0}")]
    Message(String),

    #[error("Timeout")]
    Timeout,

    #[error("Peer error: {0}")]
    Peer(String),

    #[error("Yield error: {0}")]
    Yield(#[from] YieldError),

    #[error("Task cancelled")]
    TaskCancelled(#[from] TaskErrors)
}

impl From<SignalingError> for WebRtcClientError {
    fn from(err: SignalingError) -> Self {
        WebRtcClientError::Signaling(err.to_string())
    }
}

impl From<IceError> for WebRtcClientError {
    fn from(err: IceError) -> Self {
        WebRtcClientError::Connection(err.to_string())
    }
}

impl From<WebError> for WebRtcClientError {
    fn from(err: WebError) -> Self {
        WebRtcClientError::Connection(err.to_string())
    }
}

impl From<shared::errors::CoreError> for WebRtcClientError {
    fn from(err: shared::errors::CoreError) -> Self {
        WebRtcClientError::Transfer(err.to_string())
    }
}
