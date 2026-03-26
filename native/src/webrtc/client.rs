use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use futures::channel::mpsc;
use futures::SinkExt;
use futures_util::select_biased;
use futures_util::stream::StreamExt;
use futures_util::FutureExt;
use prost::Message;
use str0m::change::SdpOffer;
use str0m::channel::{ChannelConfig, ChannelId};
use str0m::net::{Protocol, Receive};
use str0m::{Candidate, Event, IceConnectionState, Input, Output, Rtc, RtcConfig};
use str0m::rtp::RawPacket;
use thiserror::Error;
use tokio::sync::{OnceCell, watch};
use core_services::utils::yield_container::{YieldContainer, YieldError};
use devlog_sdk::distributed_id::gen_id;
use schema::devlog::bitbridge::peer_message_body::{Request, Response};
use schema::devlog::bitbridge::*;
use schema::devlog::rpc_signalling::server::OfferMessage;

use shared::app::operations::p2p::P2POperationOutput;
use shared::app::operations::transfer::TransferOperationOutput;
use shared::app::operations::CoreOperationOutput;
use shared::entities::local_resource::{LocalResource, LocalResourcePath, ResourceType};
use shared::entities::peer::Peer;
use shared::entities::transfer_session::TransferProgress;
use shared::errors::CoreError;
use shared::protocol::webrtc::errors::WebRtcErrors;
use shared::protocol::webrtc::message_channel::DirectMessageChannel;
use shared::protocol::webrtc::transfer::{TransferDelimiterShema, TransfersContext};
use shared::protocol::webrtc::fec::{FecAction, FecSender, CHUNK_SIZE, DATA_SHARDS_DEFAULT};
use shared::utils::compression::is_compressible;
use shared::repository::local_resource::LocalResourceRepository;
use shared::repository::transfer_session::TransferSessionRepository;
use shared::shell::api::CoreRequest;
use schema::devlog::bitbridge::fec_feedback::Feedback;

use crate::webrtc::ice::IceAgent;
use crate::webrtc::signalling::SignalingClient;
use crate::webrtc::socket::{SyncUdpSocket, SyncUdpSocketError};

const TOTAL_CHANNELS: usize = 3;

const fn channel_id(raw: usize) -> ChannelId {
    unsafe { std::mem::transmute(raw) }
}

const RELIABLE_DATA_CHANNEL_ID: ChannelId = channel_id(1);
const UNRELIABLE_DATA_CHANNEL_ID: ChannelId = channel_id(2);
const UNORDERED_MSG_CHANNEL_ID: ChannelId = channel_id(3);

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

    #[error("Socket error: {0}")]
    Socket(#[from] SyncUdpSocketError),

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
}

impl From<WebRtcErrors> for WebRtcClientError {
    fn from(err: WebRtcErrors) -> Self {
        WebRtcClientError::Shared(format!("{err}"))
    }
}

impl From<WebRtcClientError> for CoreError {
    fn from(err: WebRtcClientError) -> Self {
        CoreError::Network(format!("WebRtcClient {err:?}"))
    }
}

pub struct WebRtcClient {
    reliable_data_tx: OnceCell<mpsc::Sender<Box<[u8]>>>,
    unreliable_data_tx: OnceCell<mpsc::Sender<Box<[u8]>>>,

    msg_channel: OnceCell<DirectMessageChannel>,

    rtc: YieldContainer<Rtc>,
    socket: SyncUdpSocket,
    local_addr: SocketAddr,

    peer: OnceCell<Peer>,
    transfers_context: TransfersContext,
    core_request: OnceCell<CoreRequest>,
    resource_repo: Arc<dyn LocalResourceRepository>,
    transfer_session_repo: Arc<dyn TransferSessionRepository>,

    outbound_packet_sender: OnceCell<mpsc::Sender<(u16, Box<[u8]>, bool)>>,
    transfer_feedback_sender: OnceCell<mpsc::UnboundedSender<Feedback>>,
}

impl WebRtcClient {
    pub async fn connect(
        offer_message: OfferMessage,
        socket: SyncUdpSocket,
        signalling: SignalingClient,
        request_id: String,
        ice_agent: IceAgent,
        resource_repo: Arc<dyn LocalResourceRepository>,
        transfer_session_repo: Arc<dyn TransferSessionRepository>,
    ) -> Result<Arc<Self>, WebRtcClientError> {
        let mut rtc = RtcConfig::new().build(Instant::now());

        let local_addr = socket.local_addr()?;
        let host_candidate = Candidate::host(local_addr, "udp")
            .map_err(|e| WebRtcClientError::Signalling(format!("{e}")))?;
        rtc.add_local_candidate(host_candidate);
        log::info!("[webrtc-client] Added host candidate: {local_addr}");

        ice_agent.gather_candidates(&mut rtc, local_addr).await;

        let mut api = rtc.direct_api();
        api.create_data_channel(ChannelConfig {
            label: "reliable-data".into(),
            ordered: true,
            negotiated: Some(1),
            ..Default::default()
        });
        api.create_data_channel(ChannelConfig {
            label: "unreliable-data".into(),
            ordered: false,
            negotiated: Some(2),
            ..Default::default()
        });
        api.create_data_channel(ChannelConfig {
            label: "unordered-msg".into(),
            ordered: false,
            negotiated: Some(3),
            ..Default::default()
        });

        let offer = SdpOffer::from_sdp_string(&offer_message.sdp)
            .map_err(|e| WebRtcClientError::SdpParse(format!("{e}")))?;

        let answer = rtc
            .sdp_api()
            .accept_offer(offer)
            .map_err(WebRtcClientError::Rtc)?;

        log::info!("[webrtc-client] SDP answer created with all local candidates");

        signalling
            .send_answer(answer.to_sdp_string(), &request_id)
            .await
            .map_err(|e| WebRtcClientError::Signalling(format!("{e}")))?;

        log::info!("[webrtc-client] Answer sent, waiting for connection and all channels");

        let mut channels_opened: usize = 0;
        let mut is_connected = false;

        let socket = socket;
        let mut buf = vec![0u8; 2000];
        loop {
            let timeout = {
                let output = rtc.poll_output()?;
                match output {
                    Output::Timeout(t) => t,
                    Output::Transmit(t) => {
                        socket.send_to(&t.contents, t.destination).await?;
                        continue;
                    }
                    Output::Event(e) => {
                        match &e {
                            Event::Connected => {
                                log::info!("[webrtc-client] Connected");
                                is_connected = true;
                            }
                            Event::ChannelOpen(_, label) => {
                                channels_opened += 1;
                                log::info!(
                                    "[webrtc-client] Channel {} opened (label: {})",
                                    channels_opened,
                                    label
                                );
                            }
                            Event::IceConnectionStateChange(state) => {
                                log::info!("[webrtc-client] ICE state: {:?}", state);
                                if matches!(state, IceConnectionState::Disconnected) {
                                    return Err(WebRtcClientError::Signalling(
                                        "Peer disconnected during setup".into(),
                                    ));
                                }
                            }
                            _ => {}
                        }

                        if is_connected && channels_opened >= TOTAL_CHANNELS {
                            log::info!("[webrtc-client] All channels open, ready");

                            let client = Arc::new(Self {
                                rtc: YieldContainer::new(rtc),
                                socket,
                                local_addr,
                                unreliable_data_tx: Default::default(),
                                reliable_data_tx: Default::default(),
                                msg_channel: Default::default(),
                                peer: OnceCell::new(),
                                transfers_context: TransfersContext::new(),
                                core_request: OnceCell::new(),
                                resource_repo,
                                transfer_session_repo,
                                outbound_packet_sender: Default::default(),
                                transfer_feedback_sender: Default::default(),
                            });

                            return Ok(client);
                        }
                        continue;
                    }
                }
            };

            let duration = timeout.saturating_duration_since(Instant::now());
            if duration.is_zero() {
                rtc.handle_input(Input::Timeout(Instant::now()))?;
                continue;
            }

            tokio::select! {
                result = {
                    socket.recv_any(&mut buf)
                } => {
                    let (n, source) = result?;
                    let receive = Receive::new(Protocol::Udp, source, local_addr, &buf[..n])
                        .map_err(|e| WebRtcClientError::Signalling(format!("{e}")))?;
                    rtc.handle_input(Input::Receive(Instant::now(), receive))?;
                }
                _ = tokio::time::sleep(duration) => {
                    rtc.handle_input(Input::Timeout(Instant::now()))?;
                }
            }
        }
    }

    pub async fn run(self: Arc<Self>) -> Result<(), WebRtcClientError> {
        let mut buf = vec![0u8; 2000];
        let mut rtc = self.rtc.retrieve().await?;

        let (msg_tx, mut msg_rx) = mpsc::channel::<Box<[u8]>>(64);
        let (reliable_tx, mut reliable_data_rx) = mpsc::channel::<Box<[u8]>>(64);
        let (unreliable_tx, mut unreliable_data_rx) = mpsc::channel::<Box<[u8]>>(64);
        let (outbound_tx, outbound_rx) = mpsc::channel::<(u16, Box<[u8]>, bool)>(64);
        let (feedback_tx, feedback_rx) = mpsc::unbounded::<Feedback>();

        let _ = self.msg_channel.set(DirectMessageChannel::new(msg_tx));
        let _ = self.reliable_data_tx.set(reliable_tx);
        let _ = self.unreliable_data_tx.set(unreliable_tx);
        let _ = self.outbound_packet_sender.set(outbound_tx);
        let _ = self.transfer_feedback_sender.set(feedback_tx);

        let mut reliable_tx_clone = self.reliable_data_tx.get().cloned().unwrap();
        let mut unreliable_tx_clone = self.unreliable_data_tx.get().cloned().unwrap();

        let this = self.clone();
        let sending_handle = tokio::spawn(async move {
            this.sending_loop(&mut reliable_tx_clone, &mut unreliable_tx_clone, outbound_rx, feedback_rx).await;
        });

        let run_loop = async {
            loop {
                let is_alive = rtc.is_alive();

                if !is_alive {
                    return Ok::<(), WebRtcClientError>(());
                }

                let timeout: Instant = {
                    loop {
                        match rtc.poll_output()? {
                            Output::Timeout(t) => break t,
                            Output::Transmit(t) => {
                                self.socket.send_to(&t.contents, t.destination).await?;
                            }
                            Output::Event(e) => match e {
                                Event::IceConnectionStateChange(state) => {
                                    log::info!("[webrtc-client] ICE state: {:?}", state);
                                    if matches!(state, IceConnectionState::Disconnected) {
                                        rtc.disconnect();
                                    }
                                }
                                Event::ChannelData(data) => {
                                    if data.id == UNORDERED_MSG_CHANNEL_ID {
                                        if let Ok(msg) = PeerMessageBody::decode(&data.data[..]) {
                                            let request_id = msg.request_id;
                                            if let Some(response) = msg.response {
                                                self.msg_channel().notify_response(request_id, response).await;
                                            } else if let Some(request) = msg.request {
                                                self.process_message_packet(request_id, request).await;
                                            }
                                        }
                                    }
                                }
                                _ => {}
                            },
                        }
                    }
                };

                let duration = timeout
                    .saturating_duration_since(Instant::now())
                    .max(Duration::from_millis(1));

                tokio::select! {
                result = {
                    self.socket.recv_any(&mut buf)
                } => {
                    let (n, source) = result?;
                    let Ok(receive) = Receive::new(Protocol::Udp, source, self.local_addr, &buf[..n]) else {
                        continue;
                    };

                    rtc.handle_input(Input::Receive(Instant::now(), receive))?;
                }
                _ = tokio::time::sleep(duration) => {

                    rtc.handle_input(Input::Timeout(Instant::now()))?;
                }
                Some(data) = msg_rx.next() => {

                    if let Some(mut ch) = rtc.channel(UNORDERED_MSG_CHANNEL_ID) {
                        ch.write(true, &data).ok();
                    }
                }
                Some(data) = reliable_data_rx.next() => {

                    if let Some(mut ch) = rtc.channel(RELIABLE_DATA_CHANNEL_ID) {
                        ch.write(true, &data).ok();
                    }
                }
                Some(data) = unreliable_data_rx.next() => {
                    if let Some(mut ch) = rtc.channel(UNRELIABLE_DATA_CHANNEL_ID) {
                        ch.write(true, &data).ok();
                    }
                }
            }
            };

            Ok(())
        };

        tokio::select! {
            result = run_loop => {
                log::info!("[webrtc-client] Run loop terminated, stopping run() {:?}", result);
                result
            },
            result = sending_handle => {
                log::info!("[webrtc-client] Sending loop terminated, stopping run() {:?}", result);
                Ok(())
            },
        }?;

        Ok(())
    }

    pub fn start_core_stream(&self, core_request: CoreRequest) {
        let _ = self.core_request.set(core_request);
    }

    pub fn core_request(&self) -> Option<&CoreRequest> {
        self.core_request.get()
    }

    pub async fn peer_id(&self) -> Option<String> {
        self.peer.get().map(|p| p.id().to_string())
    }

    pub async fn peer_entity(&self) -> Option<Peer> {
        self.peer.get().cloned()
    }

    pub async fn introduce(&self, current_user: &Peer) -> Result<(), WebRtcClientError> {
        let introduce_request = IntroduceRequestMessage {
            mine: PeerMessage::from(current_user.clone()),
        };

        log::info!("[webrtc-client] Sending introduce request");
        let response = self
            .msg_channel().send(Request::IntroduceRequest(introduce_request), None)
            .await?;

        match response {
            Response::IntroduceResponse(resp) => {
                let peer: Peer = resp.peer.into();
                log::info!("[webrtc-client] Received introduce response from {:?}", peer.id());
                self.peer.set(peer).unwrap();
                Ok(())
            }
            _ => Err(WebRtcClientError::FailedToIntroducePeer),
        }
    }

    pub async fn from_introduce_request(
        &self,
        request_id: String,
        msg: IntroduceRequestMessage,
        current_user: &Peer,
    ) -> Result<(), WebRtcClientError> {
        log::info!("[webrtc-client] Received introduce request from {:?}", msg.mine.peer_id);

        let response = Response::IntroduceResponse(IntroduceResponseMessage {
            peer: PeerMessage::from(current_user.clone()),
        });

        self.msg_channel().send_response(request_id, response).await?;

        let peer: Peer = msg.mine.into();
        self.peer.set(peer).unwrap();
        Ok(())
    }

    pub fn msg_channel(&self) -> &DirectMessageChannel {
        self.msg_channel.get().unwrap()
    }

    pub async fn process_message_packet(&self, request_id: String, msg: Request) {
        match msg {
            Request::CancelRequest(request) => {
                log::info!("[webrtc-client] Received cancel request {:?}", request);
                if let Some(resource_id) = request.resource_id {
                    self.transfers_context.cancel_resource(request.session_id, resource_id).await;
                } else {
                    self.transfers_context.cancel_transfer(request.session_id).await;
                }
            }
            Request::ViewSessionRequest(req) => {
                log::info!("[webrtc-client] Received view session request for order_id {}", req.order_id);
                let peer_id = self.peer.get().map(|p| p.id().to_string()).unwrap_or_default();
                let response = CoreOperationOutput::P2P(P2POperationOutput::ReceivedViewSessionRequest {
                    peer_id,
                    request_id,
                    order_id: req.order_id,
                    password: req.password,
                });

                if let Some(core_request) = self.core_request() {
                    core_request.response(response).await;
                }
            }
            Request::DownloadResourceRequest(req) => {
                let peer_id = self.peer.get().map(|p| p.id().to_string()).unwrap_or_default();
                let response = CoreOperationOutput::P2P(P2POperationOutput::ReceivedDownloadRequest {
                    peer_id,
                    session_order_id: req.session_order_id,
                    resource_order_id: req.resource_order_id,
                    transfer_id: req.transfer_id as u16,
                });
                if let Some(core_request) = self.core_request() {
                    core_request.response(response).await;
                }
            }
            Request::ResourceNotification(notification) => {
                let session_order_id = notification.session_order_id;
                log::info!(
                    "[webrtc-client] Received resource notification for session {}",
                    session_order_id
                );
                if let Some(resource_proto) = notification.resource {
                    let mut resource = LocalResource {
                        order_id: resource_proto.order_id,
                        name: resource_proto.name,
                        size: resource_proto.size as u64,
                        path: LocalResourcePath::RelativePath {
                            path: format!(
                                "received/session_{}/resource_{}",
                                session_order_id, resource_proto.order_id
                            ),
                            is_private: false,
                        },
                        thumbnail_path: None,
                        r#type: (ResourceTypeMessage::try_from(resource_proto.r#type)
                            .unwrap_or_default())
                        .try_into()
                        .unwrap_or(ResourceType::File),
                        shelf_id: 0,
                    };

                    if let Some(thumbnail_bytes) = resource_proto.thumbnail_png {
                        match self.resource_repo.save_thumbnail(thumbnail_bytes, resource.order_id).await {
                            Ok(thumbnail_path) => {
                                resource.thumbnail_path = Some(thumbnail_path);
                            }
                            Err(e) => {
                                log::warn!("[webrtc-client] Failed to save thumbnail: {:?}", e);
                            }
                        }
                    }

                    if let Some(core_request) = self.core_request() {
                        let peer_id =
                            self.peer.get().map(|p| p.id().to_string()).unwrap_or_default();
                        let response =
                            CoreOperationOutput::P2P(P2POperationOutput::ReceivedResourceNotification {
                                session_order_id,
                                resource,
                                peer_id,
                            });
                        core_request.response(response).await;
                    }
                }

                let _ = self
                    .msg_channel().send_response(request_id, Response::VoidResponse(VoidResponseMessage {}))
                    .await;
            }
            Request::FecFeedback(feedback) => {
                if let (Some(sender), Some(inner)) = (self.transfer_feedback_sender.get(), feedback.feedback) {
                    match inner {
                        fec_feedback::Feedback::Network(stats) => {
                            let _ = sender.unbounded_send(Feedback::Network(stats));
                        }
                        fec_feedback::Feedback::Missing(blocks) => {
                            let _ = sender.unbounded_send(Feedback::Missing(blocks));
                        }
                    }
                }
            }
            _ => {}
        }
    }

    pub async fn cancel_transfer(&self, session_id: u64) {
        self.transfers_context.cancel_transfer(session_id).await;
        let peer_id = self.peer.get().map(|p| p.id()).unwrap_or_default();
        log::info!("[webrtc-client] Cancelling transfer session {session_id} to peer {peer_id}");
        let _ = self
            .msg_channel().notify(Request::CancelRequest(P2pCancelSessionRequest {
                session_id,
                resource_id: None,
            }))
            .await;
    }

    pub async fn cancel_resource_transfer(&self, session_id: u64, resource_id: u64) {
        self.transfers_context.cancel_resource(session_id, resource_id).await;
        log::info!(
            "[webrtc-client] Cancelling resource {resource_id} in session {session_id}"
        );
        let _ = self.msg_channel()
            .notify(Request::CancelRequest(P2pCancelSessionRequest {
                session_id,
                resource_id: Some(resource_id),
            }))
            .await;
    }

    pub async fn send_session_detail_response(
        &self,
        request_id: String,
        session_message: Option<P2pTransferSessionMessage>,
        resources: Option<Vec<LocalResource>>,
        error: Option<CoreError>,
    ) -> Result<(), WebRtcClientError> {
        use schema::devlog::bitbridge::view_session_detail_response::Result as ResponseResult;

        log::info!("[webrtc-client] Sending session detail response");

        if let Some(error_msg) = error {
            log::error!("[webrtc-client] Session detail error: {:?}", error_msg);
            let error_result = match error_msg {
                CoreError::PeerRequestError(e) => ResponseResult::Error(e.into()),
                _ => ResponseResult::Error(PeerErrorsMessage::InvalidRequest.into()),
            };
            self.msg_channel().send_response(
                request_id,
                Response::ViewSessionResponse(ViewSessionDetailResponse {
                    result: Some(error_result),
                }),
            )
            .await?;
            return Ok(());
        }

        let Some(proto_session) = session_message else {
            return Ok(());
        };

        let response = ViewSessionDetailResponse {
            result: Some(ResponseResult::Session(proto_session.clone())),
        };

        self.msg_channel().send_response(request_id, Response::ViewSessionResponse(response))
            .await?;

        tokio::time::sleep(Duration::from_millis(100)).await;

        if let Some(resources) = resources {
            let session_order_id = proto_session.order_id;
            for resource in resources {
                self.send_resource_notification(session_order_id, resource).await?;
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        }

        Ok(())
    }

    pub async fn send_resource_notification(
        &self,
        session_order_id: u64,
        resource: LocalResource,
    ) -> Result<(), WebRtcClientError> {
        let mut resource_proto = resource.to_proto();

        if let Some(thumbnail_path) = resource.thumbnail_path.as_ref() {
            if let Ok(mut cursor) = self.resource_repo.read(thumbnail_path.clone(), 64 * 1024, false).await {
                if let Ok(bytes) = cursor.read_all().await {
                    resource_proto.thumbnail_png = Some(bytes.to_vec());
                }
            }
        }

        let notification = ResourceNotificationRequest {
            session_order_id,
            resource: Some(resource_proto),
        };

        let _ = self.msg_channel().notify(Request::ResourceNotification(notification)).await?;
        Ok(())
    }

    pub async fn stream_resource(
        &self,
        session_id: u64,
        transfer_id: u16,
        resource: LocalResource,
    ) -> Result<(), WebRtcClientError> {
        let Some(sender) = self.outbound_packet_sender.get() else {
            return Err(WebRtcClientError::Transfer("sending_loop not initialized".into()));
        };

        let mut sender = sender.clone();

        let compressed = is_compressible(&resource.name);
        let prefix = transfer_id;

        let start_delimiter = TransferDelimiterShema::start(session_id, resource.order_id, compressed);
        let start_packet = start_delimiter.as_bytes()?;
        sender.send((prefix, start_packet, true)).await
            .map_err(|e| WebRtcClientError::Transfer(format!("failed to send start delimiter: {e}")))?;

        let chunk_size = if compressed {
            (CHUNK_SIZE * DATA_SHARDS_DEFAULT - CHUNK_SIZE) as u64
        } else {
            (CHUNK_SIZE * DATA_SHARDS_DEFAULT) as u64
        };

        let mut cursor = self.resource_repo.read(resource.path.clone(), chunk_size as usize, compressed).await
            .map_err(|e| WebRtcClientError::Transfer(format!("failed to read resource: {e}")))?;

        loop {
            match cursor.c_next(None).await {
                Ok(Some((data, _raw_size))) => {
                    if data.is_empty() {
                        break;
                    }
                    let packet = data.to_vec().into_boxed_slice();
                    sender.send((prefix, packet, false)).await
                        .map_err(|e| WebRtcClientError::Transfer(format!("failed to send data packet: {e}")))?;
                }
                Ok(None) => {
                    break;
                }
                Err(e) => {
                    return Err(WebRtcClientError::Transfer(format!("failed to read chunk: {e}")));
                }
            }
        }

        let end_delimiter = TransferDelimiterShema::end(session_id, resource.order_id, compressed);
        let end_packet = end_delimiter.as_bytes()?;
        sender.send((prefix, end_packet, true)).await
            .map_err(|e| WebRtcClientError::Transfer(format!("failed to send end delimiter: {e}")))?;

        log::info!("[webrtc-client] Completed streaming resource {} for session {}", resource.order_id, session_id);
        Ok(())
    }

    async fn sending_loop(
        &self,
        reliable_tx: &mut mpsc::Sender<Box<[u8]>>,
        unreliable_tx: &mut mpsc::Sender<Box<[u8]>>,
        mut outbound_rx: mpsc::Receiver<(u16, Box<[u8]>, bool)>,
        mut feedback_rx: mpsc::UnboundedReceiver<Feedback>,
    ) {
        const MAX_BUFFER_SIZE: usize = 1024 * 1024 * 5;
        const MIN_BUFFER_SIZE: usize = CHUNK_SIZE;

        let mut fec_sender = FecSender::new(1024);
        let mut buff_counter = MAX_BUFFER_SIZE - CHUNK_SIZE;
        let mut last_peer_block_id = 0u32;
        const WINDOW_SIZE: u32 = 128;

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
                (Some((prefix, packet, reliable)), _) => {
                    let action = fec_sender.send(prefix, packet);
                    (reliable, action)
                }
                (_, Some(fb)) => {
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
                    (true, Ok(fec_sender.feedback(fb)))
                }
                _ => continue
            };

            match action {
                Ok(FecAction::Framed(frames)) => {
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
                Ok(FecAction::Retransmit(frames)) => {
                    let frame_idx = frames.iter().map(|it| it.frame_idx).collect::<Vec<_>>();
                    log::info!("Retransmit {:?} {:?}", frames.first().map(|it| it.block_id), frame_idx);
                    for frame in frames {
                        let packet = frame.serialize();
                        buff_counter += packet.len();
                        let _ = reliable_tx.send(packet).await;
                    }
                    buff_counter = buff_counter.max(MAX_BUFFER_SIZE / 2);
                }
                Ok(FecAction::Terminated) => {
                    log::info!("FEC sender terminated in sending_loop");
                    break;
                }
                Ok(FecAction::Noop) => {
                    continue;
                }
                Ok(_) | Err(_) => {
                    continue;
                }
            }

            if buff_counter >= MAX_BUFFER_SIZE {
                let timeout = Duration::from_millis(20 * fec_sender.rtt().max(100));
                tokio::time::sleep(timeout).await;
                buff_counter = MIN_BUFFER_SIZE;
            }
        }
    }
}
