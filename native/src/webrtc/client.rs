use std::collections::HashMap;
use std::net::SocketAddr;
use std::ops::DerefMut;
use std::sync::Arc;
use std::time::Instant;

use core_services::utils::yield_container::{YieldContainer, YieldError};
use futures::channel::mpsc;
use futures::SinkExt;
use futures_util::select_biased;
use futures_util::stream::StreamExt;
use prost::Message;
use schema::devlog::bitbridge::peer_message_body::{Request, Response};
use schema::devlog::bitbridge::view_session_detail_response::Result as SessionDetailResult;
use schema::devlog::bitbridge::*;
use schema::devlog::rpc_signalling::server::OfferMessage;
use socket2::{Domain, Socket, Type};
use str0m::channel::{ChannelConfig, ChannelId};
use str0m::net::{Protocol, Receive};
use str0m::{Event, IceConnectionState, Input, Output, Rtc};
use thiserror::Error;
use tokio::net::UdpSocket;
use tokio::sync::OnceCell;

use schema::devlog::bitbridge::fec_feedback::Feedback;
use shared::app::operations::p2p::P2POperationOutput;
use shared::app::operations::CoreOperationOutput;
use shared::entities::local_resource::{LocalResource, LocalResourcePath};
use shared::entities::peer::Peer;
use shared::errors::CoreError;
use shared::protocol::webrtc::errors::WebRtcErrors;
use shared::protocol::webrtc::fec::{FecAction, FecSender};
use shared::protocol::webrtc::message_channel::DirectMessageChannel;
use shared::protocol::webrtc::transfer::TransfersContext;
use shared::repository::local_resource::LocalResourceRepository;
use shared::shell::api::CoreRequest;

use crate::webrtc::ice::IceAgent;
use crate::webrtc::signalling::SignalingClient;

const TOTAL_CHANNELS: usize = 4;

const fn channel_id(raw: usize) -> ChannelId {
    unsafe { std::mem::transmute(raw) }
}

const RELIABLE_DATA_CHANNEL_ID: ChannelId = channel_id(1);
const UNRELIABLE_DATA_CHANNEL_ID: ChannelId = channel_id(2);
const UNORDERED_MSG_CHANNEL_ID: ChannelId = channel_id(3);
const ORDERED_MSG_CHANNEL_ID: ChannelId = channel_id(4);

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
    Yield(#[from] YieldError)
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
    unordered_msg_channel: OnceCell<DirectMessageChannel>,

    rtc: YieldContainer<Rtc>,
    socket: YieldContainer<UdpSocket>,
    local_addr: SocketAddr,
    local_v4_addr: Option<SocketAddr>,
    local_v6_addr: Option<SocketAddr>,

    peer: OnceCell<Peer>,
    me: Peer,
    transfers_context: TransfersContext,
    core_request: OnceCell<CoreRequest>,
    resource_repo: Arc<dyn LocalResourceRepository>,

    outbound_packet_sender: OnceCell<mpsc::Sender<(u16, Box<[u8]>, bool)>>,
    transfer_feedback_sender: OnceCell<mpsc::UnboundedSender<Feedback>>
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
        signalling: &SignalingClient,
        request_id: String,
        resource_repo: Arc<dyn LocalResourceRepository>
    ) -> Result<Arc<Self>, WebRtcClientError> {
        let Some(signalling_id) = me.signalling_id.clone() else {
            return Err(WebRtcClientError::Shared("Peer not introduced".to_string()));
        };

        let config = match signalling.fetch_relay_config(&signalling_id).await {
            Ok(c) => c,
            Err(e) => {
                log::warn!(
                    "[webrtc-client] Failed to fetch relay config ({}), proceeding without TURN relay",
                    e
                );
                schema::devlog::rpc_signalling::server::IceConfig::default()
            }
        };

        let socket = Socket::new(Domain::IPV6, Type::DGRAM, Some(socket2::Protocol::UDP))?;
        socket.set_only_v6(false)?;
        socket.set_nonblocking(true)?;
        socket.bind(&"[::]:0".parse::<SocketAddr>().unwrap().into())?;
        let std_socket: std::net::UdpSocket = socket.into();
        let socket = UdpSocket::from_std(std_socket)?;
        let socket = Arc::new(socket);

        let local_addr = socket.local_addr()?;

        let (candidates, _relay_client) = IceAgent::gather_candidates(socket.clone(), &config)
            .await
            .map_err(|e| WebRtcClientError::Signalling(format!("{e}")))?;

        let mut rtc = Rtc::new(Instant::now());
        let mut local_v4_addr = None;
        let mut local_v6_addr = None;
        for candidate in candidates {
            if candidate.addr().is_ipv4() && local_v4_addr.is_none() {
                local_v4_addr = Some(candidate.addr());
            } else if candidate.addr().is_ipv6() && local_v6_addr.is_none() {
                local_v6_addr = Some(candidate.addr());
            }
            rtc.add_local_candidate(candidate);
        }

        let offer_sdp = IceAgent::resolve_remote_candidates(&offer_message.sdp).await;
        log::info!("Received offer sdp: {offer_sdp}");
        let offer = str0m::change::SdpOffer::from_sdp_string(&offer_sdp).map_err(|e| WebRtcClientError::SdpParse(format!("{e}")))?;

        let answer = rtc.sdp_api().accept_offer(offer).map_err(WebRtcClientError::Rtc)?;

        let mut api = rtc.sdp_api();
        api.add_channel_with_config(ChannelConfig {
            label: "reliable".to_string(),
            ordered: true,
            negotiated: Some(1),
            ..Default::default()
        });

        api.add_channel_with_config(ChannelConfig {
            label: "unreliable".to_string(),
            ordered: false,
            negotiated: Some(2),
            ..Default::default()
        });

        api.add_channel_with_config(ChannelConfig {
            label: "unordered_msg".to_string(),
            ordered: false,
            negotiated: Some(3),
            ..Default::default()
        });

        api.add_channel_with_config(ChannelConfig {
            label: "ordered_msg".to_string(),
            ordered: true,
            negotiated: Some(4),
            ..Default::default()
        });

        signalling
            .send_answer(answer.to_sdp_string(), &request_id)
            .await
            .map_err(|e| WebRtcClientError::Signalling(format!("{e}")))?;

        log::info!("[webrtc-client] Answer sent, waiting for connection and all channels");

        let client = Arc::new(Self {
            me,
            rtc: YieldContainer::new(rtc),
            socket: YieldContainer::new(Arc::try_unwrap(socket).expect("socket Arc should have single owner after gather")),
            local_addr,
            local_v4_addr,
            local_v6_addr,
            unreliable_data_tx: Default::default(),
            reliable_data_tx: Default::default(),
            msg_channel: Default::default(),
            unordered_msg_channel: Default::default(),
            peer: OnceCell::new(),
            transfers_context: TransfersContext::new(),
            core_request: OnceCell::new(),
            resource_repo,
            outbound_packet_sender: Default::default(),
            transfer_feedback_sender: Default::default()
        });

        // Run the event loop until all data channels are established
        client.clone().run(true).await?;

        Ok(client)
    }

    pub async fn run(self: Arc<Self>, initial_connection: bool) -> Result<(), WebRtcClientError> {
        let mut rtc_container = self.rtc.retrieve().await?;
        let rtc = rtc_container.deref_mut();

        let mut socket_container = self.socket.retrieve().await?;
        let socket = socket_container.deref_mut();

        // Only set up mpsc channels and sending loop for normal (non-initial) run
        let (mut ordered_msg_rx, mut unordered_msg_rx, mut reliable_data_rx, mut unreliable_data_rx) = if !initial_connection {
            let (ordered_msg_tx, ordered_msg_rx) = mpsc::channel::<Box<[u8]>>(64);
            let (unordered_msg_tx, unordered_msg_rx) = mpsc::channel::<Box<[u8]>>(64);
            let (reliable_tx, reliable_data_rx) = mpsc::channel::<Box<[u8]>>(64);
            let (unreliable_tx, unreliable_data_rx) = mpsc::channel::<Box<[u8]>>(64);
            let (outbound_tx, outbound_rx) = mpsc::channel::<(u16, Box<[u8]>, bool)>(64);
            let (feedback_tx, feedback_rx) = mpsc::unbounded::<Feedback>();

            let _ = self.msg_channel.set(DirectMessageChannel::new(ordered_msg_tx));
            let _ = self.unordered_msg_channel.set(DirectMessageChannel::new(unordered_msg_tx));
            let _ = self.reliable_data_tx.set(reliable_tx);
            let _ = self.unreliable_data_tx.set(unreliable_tx);
            let _ = self.outbound_packet_sender.set(outbound_tx);
            let _ = self.transfer_feedback_sender.set(feedback_tx);

            let mut reliable_tx_clone = self.reliable_data_tx.get().cloned().unwrap();
            let mut unreliable_tx_clone = self.unreliable_data_tx.get().cloned().unwrap();

            let this = self.clone();
            tokio::spawn(async move {
                this.sending_loop(&mut reliable_tx_clone, &mut unreliable_tx_clone, outbound_rx, feedback_rx).await;
            });

            (Some(ordered_msg_rx), Some(unordered_msg_rx), Some(reliable_data_rx), Some(unreliable_data_rx))
        } else {
            (None, None, None, None)
        };

        let mut channels_opened: usize = 0;
        let mut is_connected = false;
        let mut buf = vec![0u8; 2000];

        loop {
            if !initial_connection && !rtc.is_alive() {
                return Ok(());
            }

            let timeout: Instant = {
                loop {
                    match rtc.poll_output()? {
                        Output::Timeout(t) => break t,
                        Output::Transmit(t) => {
                            let dest = to_v6_mapped(t.destination);
                            if let Err(e) = socket.send_to(&t.contents, dest).await {
                                log::warn!("[webrtc-client] Failed to send to {}: {}", dest, e);
                            }
                        }
                        Output::Event(e) => match e {
                            Event::Connected if initial_connection => {
                                log::info!("[webrtc-client] Connected");
                                is_connected = true;
                            }
                            Event::ChannelOpen(_, label) if initial_connection => {
                                channels_opened += 1;
                                log::info!("[webrtc-client] Channel {} opened (label: {})", channels_opened, label);

                                if is_connected && channels_opened >= TOTAL_CHANNELS {
                                    log::info!("[webrtc-client] All channels open, ready");
                                    return Ok(());
                                }
                            }
                            Event::IceConnectionStateChange(state) => {
                                log::info!("[webrtc-client] ICE state: {:?}", state);
                                if matches!(state, IceConnectionState::Disconnected) {
                                    if initial_connection {
                                        return Err(WebRtcClientError::Signalling("Peer disconnected during setup".into()));
                                    }
                                    rtc.disconnect();
                                }
                            }
                            Event::ChannelData(data) if !initial_connection => {
                                if data.id == ORDERED_MSG_CHANNEL_ID {
                                    if let Ok(msg) = PeerMessageBody::decode(&data.data[..]) {
                                        let request_id = msg.request_id;
                                        if let Some(response) = msg.response {
                                            self.msg_channel().notify_response(request_id, response).await;
                                        } else if let Some(request) = msg.request {
                                            self.process_message_packet(request_id, request).await;
                                        }
                                    }
                                } else if data.id == UNORDERED_MSG_CHANNEL_ID {
                                    if let Ok(msg) = PeerMessageBody::decode(&data.data[..]) {
                                        if let Some(Request::FecFeedback(feedback)) = msg.request {
                                            if let (Some(sender), Some(inner)) =
                                                (self.transfer_feedback_sender.get(), feedback.feedback)
                                            {
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
                            _ => {}
                        }
                    }
                }
            };

            let duration = timeout.saturating_duration_since(Instant::now());
            if duration.is_zero() {
                rtc.handle_input(Input::Timeout(Instant::now()))?;
                continue;
            }

            tokio::select! {
                res = socket.recv_from(&mut buf) => {
                    if let Ok((n, mut source)) = res {
                        source = from_v6_mapped(source);
                        let local = if source.is_ipv4() {
                            self.local_v4_addr.unwrap_or(self.local_addr)
                        } else {
                            self.local_v6_addr.unwrap_or(self.local_addr)
                        };
                        match Receive::new(Protocol::Udp, source, local, &buf[..n]) {
                            Ok(receive) => {
                                if let Err(e) = rtc.handle_input(Input::Receive(Instant::now(), receive)) {
                                    log::trace!("[webrtc-client] Input handle packet drop: {}", e);
                                }
                            }
                            Err(e) => {
                                log::trace!("[webrtc-client] Failed to parse Receive: {}", e);
                            }
                        }
                    }
                }
                _ = tokio::time::sleep(duration) => {
                    rtc.handle_input(Input::Timeout(Instant::now()))?;
                }
                Some(data) = async { ordered_msg_rx.as_mut()?.next().await }, if !initial_connection => {
                    if let Some(mut ch) = rtc.channel(ORDERED_MSG_CHANNEL_ID) {
                        ch.write(true, &data).ok();
                    }
                }
                Some(data) = async { unordered_msg_rx.as_mut()?.next().await }, if !initial_connection => {
                    if let Some(mut ch) = rtc.channel(UNORDERED_MSG_CHANNEL_ID) {
                        ch.write(true, &data).ok();
                    }
                }
                Some(data) = async { reliable_data_rx.as_mut()?.next().await }, if !initial_connection => {
                    if let Some(mut ch) = rtc.channel(RELIABLE_DATA_CHANNEL_ID) {
                        ch.write(true, &data).ok();
                    }
                }
                Some(data) = async { unreliable_data_rx.as_mut()?.next().await }, if !initial_connection => {
                    if let Some(mut ch) = rtc.channel(UNRELIABLE_DATA_CHANNEL_ID) {
                        ch.write(true, &data).ok();
                    }
                }
            }
        }
    }

    pub fn start_core_stream(&self, core_request: CoreRequest) {
        let _ = self.core_request.set(core_request);
    }

    pub fn core_request(&self) -> Option<&CoreRequest> {
        self.core_request.get()
    }

    pub async fn peer_id(&self) -> Option<String> {
        self.peer.get().map(|p| p.id.clone())
    }

    pub async fn peer_entity(&self) -> Option<Peer> {
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

    pub async fn stream_resource(&self, session_id: u64, transfer_id: u16, _resource: LocalResource) -> Result<(), WebRtcClientError> {
        self.transfers_context.start_transfer(session_id, transfer_id.to_string()).await;
        Ok(())
    }

    pub async fn send_resource_notification(&self, session_id: u64, resource: LocalResource) -> Result<(), WebRtcClientError> {
        let notification = ResourceNotificationRequest {
            session_order_id: session_id,
            resource: Some(resource.to_proto())
        };

        self.msg_channel()
            .send(Request::ResourceNotification(notification), None)
            .await
            .map(|_| ())
            .map_err(|e| WebRtcClientError::MessageChannel(format!("{e}")))?;

        Ok(())
    }

    pub async fn send_session_detail_response(
        &self,
        request_id: String,
        session_message: Option<P2pTransferSessionMessage>,
        resource_updated: Option<ResourceMessage>,
        error: Option<PeerErrorsMessage>
    ) -> Result<(), WebRtcClientError> {
        let response_result = if let Some(session) = session_message {
            Some(SessionDetailResult::Session(session))
        } else if let Some(resource) = resource_updated {
            Some(SessionDetailResult::ResourceUpdated(resource))
        } else {
            error.map(|err| SessionDetailResult::Error(err as i32))
        };

        let session_detail = ViewSessionDetailResponse { result: response_result };

        self.msg_channel().notify_response(request_id, Response::ViewSessionResponse(session_detail)).await;
        Ok(())
    }

    pub async fn cancel_transfer(&self, session_id: u64) {
        self.transfers_context.cancel_transfer(session_id).await;
    }

    pub async fn cancel_resource_transfer(&self, session_id: u64, resource_id: u64) {
        self.transfers_context.cancel_resource(session_id, resource_id).await;
    }

    fn msg_channel(&self) -> &DirectMessageChannel {
        self.msg_channel.get().expect("Message channel not initialized")
    }

    pub async fn process_message_packet(&self, request_id: String, request: Request) {
        log::info!("[webrtc-client] Received message packet: {:?}", request);

        if let Some(core) = self.core_request.get() {
            match request {
                Request::ViewSessionRequest(msg) => {
                    core.response(CoreOperationOutput::P2P(P2POperationOutput::ReceivedViewSessionRequest {
                        peer_id: self.peer.get().unwrap().id.clone(),
                        request_id,
                        password: msg.password,
                        order_id: msg.order_id
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
                }
                _ => {}
            }
        }
    }

    async fn sending_loop(
        &self,
        reliable_tx: &mut mpsc::Sender<Box<[u8]>>,
        unreliable_tx: &mut mpsc::Sender<Box<[u8]>>,
        mut outbound_rx: mpsc::Receiver<(u16, Box<[u8]>, bool)>,
        mut feedback_rx: mpsc::UnboundedReceiver<Feedback>
    ) {
        let mut fec_senders: HashMap<u16, FecSender> = HashMap::new();

        loop {
            select_biased! {
                res = outbound_rx.next() => {
                    if let Some((transfer_id, data, is_reliable)) = res {
                        if is_reliable {
                            let _ = reliable_tx.send(data).await;
                        } else {
                            let fec = fec_senders.entry(transfer_id).or_insert_with(|| FecSender::new(64));

                            if let Ok(FecAction::Framed(frames)) = fec.send(0, data) {
                                for frame in frames {
                                    let _ = unreliable_tx.send(frame.serialize()).await;
                                }
                            }
                        }
                    }
                }
                res = feedback_rx.next() => {
                    if let Some(feedback) = res {
                        match feedback {
                            Feedback::Network(stats) => {
                                // Since transfer_id is missing from NetworkStats proto, we apply to all active senders
                                for fec in fec_senders.values_mut() {
                                    if let Some(rtt) = stats.rtt {
                                        fec.set_rtt(rtt as u64);
                                    }
                                }
                            }
                            Feedback::Missing(blocks) => {
                                // Since transfer_id is missing from MissingBlocks proto, we apply to all active senders
                                // and let FecSender's inner logic handle block_id validation.
                                for fec in fec_senders.values_mut() {
                                    let action = fec.feedback(schema::devlog::bitbridge::fec_feedback::Feedback::Missing(blocks.clone()));
                                    if let FecAction::Retransmit(frames) = action {
                                        for frame in frames {
                                            let _ = unreliable_tx.send(frame.serialize()).await;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                complete => break,
            }
        }
    }
}

fn to_v6_mapped(addr: SocketAddr) -> SocketAddr {
    match addr {
        SocketAddr::V4(v4) => SocketAddr::new(v4.ip().to_ipv6_mapped().into(), v4.port()),
        v6 => v6
    }
}

fn from_v6_mapped(addr: SocketAddr) -> SocketAddr {
    match addr {
        SocketAddr::V6(v6) => {
            let octets = v6.ip().octets();
            if octets[0..12] == [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xff, 0xff] {
                let v4 = std::net::Ipv4Addr::new(octets[12], octets[13], octets[14], octets[15]);
                SocketAddr::new(v4.into(), v6.port())
            } else {
                addr
            }
        }
        _ => addr
    }
}
