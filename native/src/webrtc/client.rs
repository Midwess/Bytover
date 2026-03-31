use std::ops::DerefMut;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use core_services::utils::cancellation::{FutureExtension, TaskErrors};
use core_services::utils::yield_container::{YieldContainer, YieldError};
use futures::channel::mpsc;
use futures::SinkExt;
use futures_util::stream::StreamExt;
use futures_util::{select_biased, FutureExt};
use prost::Message;
use schema::devlog::bitbridge::peer_message_body::{Request, Response};
use schema::devlog::bitbridge::view_session_detail_response::Result as SessionDetailResult;
use schema::devlog::bitbridge::*;
use schema::devlog::rpc_signalling::server::OfferMessage;
use thiserror::Error;
use tokio::sync::OnceCell;

use schema::devlog::bitbridge::fec_feedback::Feedback;
use shared::app::operations::p2p::P2POperationOutput;
use shared::app::operations::CoreOperationOutput;
use shared::entities::local_resource::{LocalResource, LocalResourcePath, ResourceType};
use shared::entities::peer::Peer;
use shared::errors::CoreError;
use shared::protocol::webrtc::errors::WebRtcErrors;
use shared::protocol::webrtc::fec::{FecAction, FecSender, CHUNK_SIZE, DATA_SHARDS_DEFAULT};
use shared::protocol::webrtc::message_channel::DirectMessageChannel;
use shared::protocol::webrtc::transfer::{TransferDelimiterShema, TransfersContext};
use shared::repository::local_resource::LocalResourceRepository;
use shared::shell::api::CoreRequest;
use shared::utils::compression::is_compressible;

use crate::webrtc::rtc::RtcClient;
use crate::webrtc::signalling::SignallingSender;
use str0m::channel::ChannelId;
use str0m::Event;

static MAX_BUFFER_SIZE: usize = 1024 * 1024 * 5;
static MIN_BUFFER_SIZE: usize = CHUNK_SIZE;

#[derive(Debug, Error)]
pub enum WebRtcClientError {
    #[error("Rtc error: {0}")]
    Rtc(#[from] str0m::error::RtcError),

    #[error("SDP parse error: {0}")]
    SdpParse(String),

    #[error("Signalling error: {0}")]
    Signalling(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Message encode error: {0}")]
    MessageEncode(#[from] prost::EncodeError),

    #[error("Message decode error: {0}")]
    MessageDecode(#[from] prost::DecodeError),

    #[error("Failed to introduce peer")]
    FailedToIntroducePeer,

    #[error("Peer not introduced")]
    PeerNotIntroduced,

    #[error("Message channel error: {0}")]
    MessageChannel(String),

    #[error("Transfer error: {0}")]
    Transfer(String),

    #[error("Timeout waiting for response")]
    Timeout,

    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    #[error("Peer error: {0}")]
    PeerError(String),

    #[error("Cancelled")]
    Cancelled,

    #[error("Repository error: {0}")]
    Repository(String),

    #[error("WebRtc shared error: {0}")]
    Shared(String),

    #[error("Race condition {0:?}")]
    Yield(#[from] YieldError),

    #[error("Task cancelled")]
    TaskCancelled(#[from] TaskErrors)
}

impl From<WebRtcErrors> for WebRtcClientError {
    fn from(err: WebRtcErrors) -> Self {
        WebRtcClientError::Shared(format!("{err}"))
    }
}

impl From<shared::repository::errors::PersistenceError> for WebRtcClientError {
    fn from(err: shared::repository::errors::PersistenceError) -> Self {
        WebRtcClientError::Repository(format!("{err}"))
    }
}

impl From<anyhow::Error> for WebRtcClientError {
    fn from(err: anyhow::Error) -> Self {
        WebRtcClientError::Shared(format!("{err}"))
    }
}

impl From<WebRtcClientError> for CoreError {
    fn from(err: WebRtcClientError) -> Self {
        CoreError::Network(format!("WebRtcClient {err:?}"))
    }
}

pub struct WebRtcClient {
    msg_channel: OnceCell<DirectMessageChannel>,

    rtc_client: YieldContainer<RtcClient>,

    peer: OnceCell<Peer>,
    me: Peer,
    transfers_context: TransfersContext,
    core_request: OnceCell<CoreRequest>,
    resource_repo: Arc<dyn LocalResourceRepository>,
    session_id: OnceCell<u64>,

    outbound_packet_sender: OnceCell<mpsc::Sender<(u16, Vec<u8>, bool)>>,
    transfer_feedback_sender: OnceCell<mpsc::UnboundedSender<Feedback>>,

    ordered_msg_rx: YieldContainer<mpsc::Receiver<Vec<u8>>>,
    unordered_msg_rx: YieldContainer<mpsc::Receiver<Vec<u8>>>,
    reliable_data_rx: YieldContainer<mpsc::Receiver<Vec<u8>>>,
    unreliable_data_rx: YieldContainer<mpsc::Receiver<Vec<u8>>>,
    outbound_rx: YieldContainer<mpsc::Receiver<(u16, Vec<u8>, bool)>>,
    feedback_rx: YieldContainer<mpsc::UnboundedReceiver<Feedback>>,
    reliable_data_tx: YieldContainer<mpsc::Sender<Vec<u8>>>,
    unreliable_data_tx: YieldContainer<mpsc::Sender<Vec<u8>>>,
    bytes_sent_counter: Arc<AtomicUsize>
}


impl std::fmt::Debug for WebRtcClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebRtcClient").field("peer", &self.peer.get()).finish()
    }
}

impl WebRtcClient {
    pub async fn connect(
        me: Peer,
        offer_message: OfferMessage,
        signalling: SignallingSender,
        request_id: String,
        resource_repo: Arc<dyn LocalResourceRepository>
    ) -> Result<Self, WebRtcClientError> {
        let Some(signalling_id) = me.signalling_id.clone() else {
            return Err(WebRtcClientError::Shared("Peer not introduced".to_string()));
        };

        let mut rtc_client = RtcClient::connect(&signalling_id, offer_message, signalling, &request_id).await?;

        log::info!("[webrtc-client] RTC connected, creating client");

        let (ordered_msg_tx, mut ordered_msg_rx) = mpsc::channel::<Vec<u8>>(64);
        let (_unordered_msg_tx, unordered_msg_rx) = mpsc::channel::<Vec<u8>>(64);
        let (reliable_data_tx, reliable_data_rx) = mpsc::channel::<Vec<u8>>(64);
        let (unreliable_data_tx, unreliable_data_rx) = mpsc::channel::<Vec<u8>>(64);
        let (outbound_tx, outbound_rx) = mpsc::channel::<(u16, Vec<u8>, bool)>(64);
        let (feedback_tx, feedback_rx) = mpsc::unbounded::<Feedback>();

        let msg_channel = DirectMessageChannel::new(ordered_msg_tx);
        let cids = *rtc_client.channel_ids();
        let peer: OnceCell<Peer> = OnceCell::new();
        let mut introduced = false;
        // Responses queued during poll_event are buffered here and sent after the inner loop,
        // since RtcClient::send() and poll_event() both require &mut self.
        let mut pending_sends: Vec<(Vec<u8>, ChannelId)> = Vec::new();

        loop {
            while let Some(event) = rtc_client.poll_event().await? {
                if let Event::ChannelData(data) = event {
                    if data.id == cids.ordered_msg {
                        if let Ok(msg) = PeerMessageBody::decode(&data.data[..]) {
                            log::info!("Got msg {:?}", msg);
                            let request_id = msg.request_id;
                            if let Some(Response::IntroduceResponse(res)) = msg.response {
                                let p = Peer::from(res.peer);
                                log::info!("[webrtc-client] Peer introduced as {:?}", p.id);
                                let _ = peer.set(p);
                            } else if let Some(Request::IntroduceRequest(intro)) = msg.request {
                                let p = Peer::from(intro.mine);
                                log::info!("[webrtc-client] Remote peer: {:?}", p.id);
                                let _ = peer.set(p);
                                let mut bytes = vec![];
                                let response_body = PeerMessageBody {
                                    request_id,
                                    response: Some(Response::IntroduceResponse(IntroduceResponseMessage {
                                        peer: PeerMessage::from(me.clone())
                                    })),
                                    ..Default::default()
                                };
                                if response_body.encode(&mut bytes).is_ok() {
                                    pending_sends.push((bytes, cids.ordered_msg));
                                }
                            }
                        }
                    }
                }
            }

            for (data, channel_id) in pending_sends.drain(..) {
                let _ = rtc_client.send(&data, channel_id);
            }

            if peer.get().is_some() {
                break;
            }

            if !introduced {
                log::info!("[webrtc-client] Introducing self to peer");
                let _ = msg_channel
                    .notify(Request::IntroduceRequest(IntroduceRequestMessage {
                        mine: PeerMessage::from(me.clone())
                    }))
                    .await;
                introduced = true;
            }

            tokio::select! {
                _ = rtc_client.wait_for_input(rtc_client.timeout_duration()) => {}
                Some(data) = ordered_msg_rx.next() => {
                    let _ = rtc_client.send(&data, cids.ordered_msg);
                }
            }
        }

        let msg_channel_cell = OnceCell::new();
        let _ = msg_channel_cell.set(msg_channel);
        let outbound_packet_sender_cell = OnceCell::new();
        let _ = outbound_packet_sender_cell.set(outbound_tx);
        let transfer_feedback_sender_cell = OnceCell::new();
        let _ = transfer_feedback_sender_cell.set(feedback_tx);

        // rtc_client goes directly into YieldContainer here — it was never retrieved,
        // so run() can call retrieve() immediately without any async race.
        let client = Self {
            me,
            rtc_client: YieldContainer::new(rtc_client),
            msg_channel: msg_channel_cell,
            peer,
            session_id: Default::default(),
            transfers_context: TransfersContext::new(),
            core_request: OnceCell::new(),
            resource_repo,
            outbound_packet_sender: outbound_packet_sender_cell,
            transfer_feedback_sender: transfer_feedback_sender_cell,
            ordered_msg_rx: YieldContainer::new(ordered_msg_rx),
            unordered_msg_rx: YieldContainer::new(unordered_msg_rx),
            reliable_data_rx: YieldContainer::new(reliable_data_rx),
            unreliable_data_rx: YieldContainer::new(unreliable_data_rx),
            outbound_rx: YieldContainer::new(outbound_rx),
            feedback_rx: YieldContainer::new(feedback_rx),
            reliable_data_tx: YieldContainer::new(reliable_data_tx),
            unreliable_data_tx: YieldContainer::new(unreliable_data_tx),
            bytes_sent_counter: Arc::new(AtomicUsize::new(0))
        };

        Ok(client)
    }

    pub async fn run(self: Arc<Self>) -> Result<(), WebRtcClientError> {
        let mut rtc_container = self.rtc_client.retrieve().await?;
        let rtc = rtc_container.deref_mut();

        let cids = *rtc.channel_ids();

        let (ordered_msg_rx_guard, unordered_msg_rx_guard, reliable_data_rx_guard, unreliable_data_rx_guard, outbound_rx_guard, feedback_rx_guard, reliable_data_tx_guard, unreliable_data_tx_guard) = futures::join!(
            self.ordered_msg_rx.retrieve(),
            self.unordered_msg_rx.retrieve(),
            self.reliable_data_rx.retrieve(),
            self.unreliable_data_rx.retrieve(),
            self.outbound_rx.retrieve(),
            self.feedback_rx.retrieve(),
            self.reliable_data_tx.retrieve(),
            self.unreliable_data_tx.retrieve()
        );

        let mut ordered_msg_rx = ordered_msg_rx_guard?.value.take().unwrap();
        let mut unordered_msg_rx = unordered_msg_rx_guard?.value.take().unwrap();
        let mut reliable_data_rx = reliable_data_rx_guard?.value.take().unwrap();
        let mut unreliable_data_rx = unreliable_data_rx_guard?.value.take().unwrap();
        let outbound_rx = outbound_rx_guard?.value.take().unwrap();
        let feedback_rx = feedback_rx_guard?.value.take().unwrap();
        let mut reliable_data_tx = reliable_data_tx_guard?.value.take().unwrap();
        let mut unreliable_data_tx = unreliable_data_tx_guard?.value.take().unwrap();

        let (msg_tx, msg_rx) = mpsc::unbounded::<(String, Request)>();

        let this_send = self.clone();
        let mut sending_handle = tokio::spawn(async move {
            this_send
                .sending_loop(&mut reliable_data_tx, &mut unreliable_data_tx, outbound_rx, feedback_rx)
                .await;
        });

        let this_msg = self.clone();
        let mut msg_handle = tokio::spawn(async move {
            this_msg.msg_loop(msg_rx).await;
        });

        while rtc.is_alive() {
            while let Some(event) = rtc.poll_event().await? {
                match event {
                    Event::ChannelData(data) => {
                        let id = data.id;
                        let data = data.data;
                        if id == cids.ordered_msg {
                            if let Ok(msg) = PeerMessageBody::decode(&data[..]) {
                                let request_id = msg.request_id;
                                if let Some(response) = msg.response {
                                    self.msg_channel().notify_response(request_id, response).await;
                                }
                                else if let Some(request) = msg.request {
                                    log::info!("received request {request:?}");
                                    let _ = msg_tx.unbounded_send((request_id, request));
                                }
                            }
                        } else if id == cids.unordered_msg {
                            if let Ok(msg) = PeerMessageBody::decode(&data[..]) {
                                if let Some(Request::FecFeedback(feedback)) = msg.request {
                                    if let (Some(sender), Some(inner)) = (self.transfer_feedback_sender.get(), feedback.feedback) {
                                        match inner {
                                            schema::devlog::bitbridge::fec_feedback::Feedback::Network(stats) => {
                                                let _ = sender.unbounded_send(Feedback::Network(stats));
                                            }
                                            schema::devlog::bitbridge::fec_feedback::Feedback::Missing(blocks) => {
                                                let _ = sender.unbounded_send(Feedback::Missing(blocks));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Event::IceConnectionStateChange(state) => {
                        log::info!("[webrtc-client] ICE state: {:?}", state);
                    }
                    _ => {}
                }
            }

            let timeout = rtc.timeout_duration();
            tokio::select! {
                _ = rtc.wait_for_input(timeout) => {}
                Some(data) = ordered_msg_rx.next() => {
                    rtc.send(&data, cids.ordered_msg);
                }
                Some(data) = unordered_msg_rx.next() => {
                    rtc.send(&data, cids.unordered_msg);
                }
                Some(data) = reliable_data_rx.next() => {
                    rtc.send(&data, cids.reliable);
                    self.bytes_sent_counter.fetch_add(data.len(), Ordering::Relaxed);
                }
                Some(data) = unreliable_data_rx.next() => {
                    rtc.send(&data, cids.unreliable);
                    self.bytes_sent_counter.fetch_add(data.len(), Ordering::Relaxed);
                }
                result = &mut sending_handle => {
                    log::info!("[webrtc-client] sending_loop finished: {:?}", result);
                    break;
                }
                result = &mut msg_handle => {
                    log::info!("[webrtc-client] msg_loop finished: {:?}", result);
                    break;
                }
            }
        }

        self.peer_disconnected().await;
        Ok(())
    }

    async fn msg_loop(&self, mut msg_rx: mpsc::UnboundedReceiver<(String, Request)>) {
        while let Some((request_id, request)) = msg_rx.next().await {
            self.process_message_packet(request_id, request).await;
        }

        log::info!("[webrtc-client] msg_loop ended");
    }

    pub fn start_core_stream(&self, core_request: CoreRequest) {
        let _ = self.core_request.set(core_request);
    }

    pub fn core_request(&self) -> Option<&CoreRequest> {
        self.core_request.get()
    }

    pub fn peer_id(&self) -> Option<String> {
        self.peer.get().map(|p| p.id.clone())
    }

    pub fn peer_entity(&self) -> Option<Peer> {
        self.peer.get().cloned()
    }

    pub async fn introduce(&self, current_user: &Peer) -> Result<(), WebRtcClientError> {
        let introduce_request = IntroduceRequestMessage {
            mine: PeerMessage::from(current_user.clone())
        };

        log::info!("[webrtc-client] Introducing self to peer");

        let response = self
            .msg_channel()
            .send(Request::IntroduceRequest(introduce_request), None)
            .await
            .map_err(|e| WebRtcClientError::MessageChannel(format!("{e}")))?;

        match response {
            Response::IntroduceResponse(res) => {
                let peer = Peer::from(res.peer);
                log::info!("[webrtc-client] Peer introduced as {:?}", peer.id);
                let _ = self.peer.set(peer);
                Ok(())
            }
            _ => Err(WebRtcClientError::InvalidResponse("Expected Introduce response".into()))
        }
    }

    pub async fn stream_resource(&self, session_id: u64, transfer_id: u16, resource: LocalResource) -> Result<(), WebRtcClientError> {
        let resource_id = resource.order_id;
        let result = self.stream_resource_inner(session_id, transfer_id, resource).await;
        if let Err(ref e) = result {
            log::warn!("[webrtc-client] stream_resource failed for resource {resource_id}: {e:?}");
            self.cancel_resource_transfer(session_id, resource_id).await;
        }
        result
    }

    async fn stream_resource_inner(
        &self,
        session_id: u64,
        transfer_id: u16,
        resource: LocalResource
    ) -> Result<(), WebRtcClientError> {
        let resource_id = resource.order_id;
        let prefix = transfer_id;
        let resource_token = self.transfers_context.get_or_create_resource_token(session_id, resource_id).await;

        let resource_name = match resource.r#type {
            ResourceType::Folder => format!("{}.zip", &resource.name),
            _ => resource.name.clone()
        };

        log::info!("[webrtc-client] Streaming resource {resource_id} for transfer id {transfer_id}");
        let compressed = is_compressible(&resource_name);

        let start_delimiter = TransferDelimiterShema::start(session_id, resource_id, compressed);
        let start_packet = start_delimiter.as_bytes()?;
        let mut outbound_packet_sender = self
            .outbound_packet_sender
            .get()
            .ok_or(WebRtcClientError::Transfer("Outbound packet sender not initialized".into()))?
            .clone();

        outbound_packet_sender
            .send((prefix, start_packet, true))
            .with_cancel(&resource_token)
            .await?
            .map_err(|e| WebRtcClientError::Transfer(format!("Failed to send start delimiter: {e:?}")))?;

        let chunk_size = if compressed {
            (CHUNK_SIZE * DATA_SHARDS_DEFAULT - CHUNK_SIZE) as u64
        } else {
            (CHUNK_SIZE * DATA_SHARDS_DEFAULT) as u64
        };

        let mut cursor = self.resource_repo.read(resource.path.clone(), chunk_size as usize, compressed).await?;

        loop {
            match cursor.c_next(None).await? {
                Some((data, _raw_size)) => {
                    if data.is_empty() {
                        log::warn!("[webrtc-client] Cursor returned empty data");
                        break;
                    }
                    let packet = data.to_vec();
                    let size = packet.len();
                    outbound_packet_sender
                        .send((prefix, packet, false))
                        .with_cancel(&resource_token)
                        .await?
                        .map_err(|e| WebRtcClientError::Transfer(format!("Failed to send data packet: {e:?}")))?;
                    log::info!("Sent {size} bytes");
                }
                None => break
            }
        }

        let end_delimiter = TransferDelimiterShema::end(session_id, resource_id, compressed);
        let end_packet = end_delimiter.as_bytes()?;
        outbound_packet_sender
            .send((prefix, end_packet, true))
            .with_cancel(&resource_token)
            .await?
            .map_err(|e| WebRtcClientError::Transfer(format!("Failed to send end delimiter: {e:?}")))?;

        log::info!("[webrtc-client] Completed streaming resource {resource_id} for session {session_id}");
        Ok(())
    }

    pub async fn send_resource_notification(&self, session_order_id: u64, resource: LocalResource) -> Result<(), WebRtcClientError> {
        let Some(mine_session_id) = self.session_id.get() else {
            log::error!("[webrtc-client] Session id not found");
            return Err(WebRtcClientError::Transfer("Session id not found".to_string()));
        };

        if *mine_session_id != session_order_id {
            return Ok(())
        }

        let mut resource_proto = resource.to_proto();

        if let Some(thumbnail_path) = resource.thumbnail_path.as_ref() {
            if let Ok(mut thumbnail_cursor) = self.resource_repo.read(thumbnail_path.clone(), 64 * 1024, false).await {
                if let Ok(bytes) = thumbnail_cursor.read_all().await {
                    resource_proto.thumbnail_png = Some(bytes.to_vec());
                }
            }
        }

        let notification = ResourceNotificationRequest {
            session_order_id,
            resource: Some(resource_proto)
        };

        self.msg_channel()
            .notify(Request::ResourceNotification(notification))
            .await
            .map(|_| ())
            .map_err(|e| WebRtcClientError::MessageChannel(format!("{e}")))?;

        Ok(())
    }

    pub async fn send_session_detail_response(
        &self,
        request_id: String,
        session_message: Option<P2pTransferSessionMessage>,
        resources: Option<Vec<LocalResource>>,
        error: Option<CoreError>
    ) -> Result<(), WebRtcClientError> {
        if let Some(error_msg) = error {
            log::error!("[webrtc-client] Session detail error: {error_msg:?}");
            let err_result = match error_msg {
                CoreError::PeerRequestError(e) => SessionDetailResult::Error(e.into()),
                _ => SessionDetailResult::Error(PeerErrorsMessage::InvalidRequest.into())
            };
            self.msg_channel()
                .send_response(
                    request_id,
                    Response::ViewSessionResponse(ViewSessionDetailResponse { result: Some(err_result) })
                )
                .await
                .map_err(|e| WebRtcClientError::MessageChannel(format!("{e}")))?;
            return Ok(());
        }

        let Some(proto_session) = session_message else {
            return Ok(());
        };

        log::info!(
            "[webrtc-client] Sending session detail: order_id={}, password_protected={}",
            proto_session.order_id,
            proto_session.password_protected
        );

        let response = ViewSessionDetailResponse {
            result: Some(SessionDetailResult::Session(proto_session.clone()))
        };
        self.msg_channel()
            .send_response(request_id, Response::ViewSessionResponse(response))
            .await
            .map_err(|e| WebRtcClientError::MessageChannel(format!("{e}")))?;

        tokio::time::sleep(Duration::from_millis(100)).await;

        if let Some(resources) = resources {
            let session_order_id = proto_session.order_id;
            for resource in resources {
                log::debug!("[webrtc-client] Sending resource notification order_id={}", resource.order_id);
                self.send_resource_notification(session_order_id, resource).await?;
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        }

        Ok(())
    }

    pub async fn cancel_transfer(&self, session_id: u64) {
        self.transfers_context.cancel_transfer(session_id).await;
        let cancel_msg = P2pCancelSessionRequest {
            session_id,
            resource_id: None
        };
        log::info!("[webrtc-client] Cancelling transfer session {session_id}");
        let _ = self.msg_channel().notify(Request::CancelRequest(cancel_msg)).await;
    }

    pub async fn cancel_resource_transfer(&self, session_id: u64, resource_id: u64) {
        self.transfers_context.cancel_resource(session_id, resource_id).await;
        let cancel_msg = P2pCancelSessionRequest {
            session_id,
            resource_id: Some(resource_id)
        };
        log::info!("[webrtc-client] Cancelling resource {resource_id} in session {session_id}");
        let _ = self.msg_channel().notify(Request::CancelRequest(cancel_msg)).await;
    }

    pub async fn peer_disconnected(&self) {
        log::info!("[webrtc-client] Peer disconnected, cancelling all transfers");
        self.transfers_context.cancel_all_transfers().await;
    }

    fn msg_channel(&self) -> &DirectMessageChannel {
        self.msg_channel.get().expect("Message channel not initialized")
    }

    pub async fn process_message_packet(&self, request_id: String, request: Request) {
        match request {
            Request::CancelRequest(req) => {
                log::info!("[webrtc-client] Received cancel request {:?}", req);
                if let Some(resource_id) = req.resource_id {
                    self.transfers_context.cancel_resource(req.session_id, resource_id).await;
                } else {
                    self.transfers_context.cancel_transfer(req.session_id).await;
                }
            }
            Request::IntroduceRequest(msg) => {
                log::info!("[webrtc-client] Received introduce request from peer");
                let peer = Peer::from(msg.mine);
                log::info!("[webrtc-client] Remote peer: {:?}", peer.id);
                let _ = self.peer.set(peer);

                let response = IntroduceResponseMessage {
                    peer: PeerMessage::from(self.me.clone())
                };
                let _ = self.msg_channel().send_response(request_id, Response::IntroduceResponse(response)).await;
            }
            Request::FecFeedback(feedback) => {
                if let Some(feedback) = feedback.feedback {
                    if let Some(sender) = self.transfer_feedback_sender.get() {
                        let _ = sender.unbounded_send(feedback);
                    }
                }
            }
            request => {
                if let Some(core) = self.core_request.get() {
                    match request {
                        Request::ViewSessionRequest(msg) => {
                            let _ = self.session_id.set(msg.order_id);
                            core.response(CoreOperationOutput::P2P(P2POperationOutput::ReceivedViewSessionRequest {
                                peer_id: self.peer.get().unwrap().id.clone(),
                                request_id,
                                password: msg.password,
                                order_id: msg.order_id
                            }))
                            .await;
                        }
                        Request::DownloadResourceRequest(req) => {
                            core.response(CoreOperationOutput::P2P(P2POperationOutput::ReceivedDownloadRequest {
                                peer_id: self.peer.get().unwrap().id.clone(),
                                session_order_id: req.session_order_id,
                                resource_order_id: req.resource_order_id,
                                transfer_id: req.transfer_id as u16
                            }))
                            .await;
                        }
                        Request::ResourceNotification(msg) => {
                            if let Some(res) = msg.resource {
                                let resource = LocalResource {
                                    order_id: res.order_id,
                                    name: res.name.clone(),
                                    size: res.size as u64,
                                    path: LocalResourcePath::RelativePath {
                                        path: format!("received/session_{}/resource_{}", msg.session_order_id, res.order_id),
                                        is_private: false
                                    },
                                    thumbnail_path: None,
                                    r#type: ResourceTypeMessage::try_from(res.r#type).unwrap_or_default().into(),
                                    shelf_id: 0
                                };
                                core.response(CoreOperationOutput::P2P(P2POperationOutput::ReceivedResourceNotification {
                                    peer_id: self.peer.get().unwrap().id.clone(),
                                    session_order_id: msg.session_order_id,
                                    resource
                                }))
                                .await;
                            }

                            let _ = self.msg_channel().send_response(request_id, Response::VoidResponse(VoidResponseMessage {})).await;
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    async fn sending_loop(
        &self,
        reliable_tx: &mut mpsc::Sender<Vec<u8>>,
        unreliable_tx: &mut mpsc::Sender<Vec<u8>>,
        mut outbound_rx: mpsc::Receiver<(u16, Vec<u8>, bool)>,
        mut feedback_rx: mpsc::UnboundedReceiver<Feedback>
    ) {
        let mut fec_sender = FecSender::new(1024);
        let mut last_peer_block_id = 0u32;
        const WINDOW_SIZE: u32 = 128;
        let mut buff_counter = MAX_BUFFER_SIZE - CHUNK_SIZE;

        loop {
            let (packet, feedback) = {
                let can_send = fec_sender.block_id.wrapping_sub(last_peer_block_id) < WINDOW_SIZE;
                let fb_fut = feedback_rx.next().fuse();

                if can_send {
                    let reader_fut = outbound_rx.next().fuse();
                    futures::pin_mut!(fb_fut);
                    futures::pin_mut!(reader_fut);
                    select_biased! {
                        r = reader_fut => {
                            r.map(|it| (Some(it), None)).unwrap_or((None, None))
                        },
                        fb = fb_fut => {
                            fb.map(|f| (None, Some(f))).unwrap_or((None, None))
                        },
                    }
                } else {
                    futures::pin_mut!(fb_fut);
                    select_biased! {
                        fb = fb_fut => {
                            fb.map(|f| (None, Some(f))).unwrap_or((None, None))
                        },
                        _ = tokio::time::sleep(Duration::from_millis(5)).fuse() => {
                            (None, None)
                        }
                    }
                }
            };

            let (reliable, action) = match (packet, feedback) {
                (Some((prefix, packet, reliable)), _) => match fec_sender.send(prefix, packet) {
                    Ok(action) => (reliable, action),
                    Err(e) => {
                        log::error!("[webrtc-client] FEC send error: {e:?}");
                        continue;
                    }
                },
                (_, Some(fb)) => {
                    use schema::devlog::bitbridge::fec_feedback::Feedback;
                    match &fb {
                        Feedback::Network(stats) => {
                            if let Some(peer_block_id) = stats.current_block_id {
                                last_peer_block_id = last_peer_block_id.max(peer_block_id);
                            }
                        }
                        Feedback::Missing(missing) => {
                            if let Some(first_block) = missing.blocks.first() {
                                last_peer_block_id = last_peer_block_id.max(first_block.block_id);
                            }
                        }
                    }

                    (true, fec_sender.feedback(fb))
                }
                _ => continue
            };

            match action {
                FecAction::Framed(frames) => {
                    for frame in frames {
                        let packet = frame.serialize();
                        buff_counter += packet.len();
                        if reliable {
                            let _ = reliable_tx.send(packet).await;
                        } else {
                            let _ = unreliable_tx.send(packet).await;
                        }
                    }
                }
                FecAction::Retransmit(frames) => {
                    let frame_idx = frames.iter().map(|it| it.frame_idx).collect::<Vec<_>>();
                    log::info!("Retransmit {:?} {:?}", frames.first().map(|it| it.block_id), frame_idx);
                    for frame in frames {
                        let packet = frame.serialize();
                        buff_counter += packet.len();
                        let _ = reliable_tx.send(packet).await;
                    }
                    buff_counter = buff_counter.max(MAX_BUFFER_SIZE / 2);
                }
                FecAction::Terminated => {
                    log::info!("FEC sender terminated in sending_loop");
                    break;
                }
                FecAction::Noop => {
                    continue;
                }
                _ => {}
            }

            if buff_counter >= MAX_BUFFER_SIZE {
                let tick = Instant::now();
                let stats_before = self.bytes_sent_counter.load(Ordering::Relaxed);
                let timeout = Duration::from_millis(20 * fec_sender.rtt().max(100));

                let deadline = Instant::now() + timeout;
                loop {
                    let sent_since = self.bytes_sent_counter.load(Ordering::Relaxed).saturating_sub(stats_before);
                    if sent_since >= buff_counter.saturating_sub(MIN_BUFFER_SIZE) {
                        break;
                    }
                    if Instant::now() >= deadline {
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(5)).await;
                }
                buff_counter = MIN_BUFFER_SIZE;

                if let Ok(hold_delimiter) = TransferDelimiterShema::hold(1).as_bytes() {
                    if let Ok(FecAction::Framed(frames)) = fec_sender.send(0, hold_delimiter) {
                        for frame in frames {
                            let _ = reliable_tx.send(frame.serialize()).await;
                        }
                    }
                }

                let time = tick.elapsed().as_secs_f64().max(f64::MIN);
                let stats_after = self.bytes_sent_counter.load(Ordering::Relaxed);
                let total_sent = stats_after.saturating_sub(stats_before);
                let bw = (total_sent as f64 / time) as u64;
                let _bw_kbps = bw / 1000;
            }
        }
    }
}
