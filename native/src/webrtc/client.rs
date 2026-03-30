use std::ops::DerefMut;
use std::sync::Arc;
use std::time::Duration;

use core_services::utils::yield_container::{YieldContainer, YieldError};
use futures::channel::mpsc;
use futures::SinkExt;
use futures_util::select_biased;
use futures_util::stream::StreamExt;
use futures_util::FutureExt;
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
use shared::entities::local_resource::{LocalResource, LocalResourcePath};
use shared::entities::peer::Peer;
use shared::errors::CoreError;
use shared::protocol::webrtc::errors::WebRtcErrors;
use shared::protocol::webrtc::fec::{FecAction, FecSender, CHUNK_SIZE};
use shared::protocol::webrtc::message_channel::DirectMessageChannel;
use shared::protocol::webrtc::transfer::{TransfersContext, TransferDelimiterShema};
use shared::repository::local_resource::LocalResourceRepository;
use shared::shell::api::CoreRequest;

use crate::webrtc::rtc::{RtcClient, RtcEvent, ORDERED_MSG_CHANNEL_ID, UNORDERED_MSG_CHANNEL_ID};
use crate::webrtc::signalling::SignalingClient;

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

    rtc_client: YieldContainer<RtcClient>,

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

        let rtc_client = RtcClient::connect(&signalling_id, offer_message, signalling, &request_id).await?;

        log::info!("[webrtc-client] RTC connected, creating client");

        let client = Arc::new(Self {
            me,
            rtc_client: YieldContainer::new(rtc_client),
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

        Ok(client)
    }

    pub async fn run(self: Arc<Self>) -> Result<(), WebRtcClientError> {
        let mut rtc_container = self.rtc_client.retrieve().await?;
        let rtc_client = rtc_container.deref_mut();

        // Set up channels: RtcClient holds receivers, we get senders
        let channel_senders = rtc_client.setup_channels();

        let (outbound_tx, outbound_rx) = mpsc::channel::<(u16, Box<[u8]>, bool)>(64);
        let (feedback_tx, feedback_rx) = mpsc::unbounded::<Feedback>();

        let _ = self.msg_channel.set(DirectMessageChannel::new(channel_senders.ordered_msg_tx));
        let _ = self.unordered_msg_channel.set(DirectMessageChannel::new(channel_senders.unordered_msg_tx));
        let _ = self.reliable_data_tx.set(channel_senders.reliable_data_tx);
        let _ = self.unreliable_data_tx.set(channel_senders.unreliable_data_tx);
        let _ = self.outbound_packet_sender.set(outbound_tx);
        let _ = self.transfer_feedback_sender.set(feedback_tx);

        let mut reliable_tx_clone = self.reliable_data_tx.get().cloned().unwrap();
        let mut unreliable_tx_clone = self.unreliable_data_tx.get().cloned().unwrap();

        let this = self.clone();
        tokio::spawn(async move {
            this.sending_loop(&mut reliable_tx_clone, &mut unreliable_tx_clone, outbound_rx, feedback_rx).await;
        });

        loop {
            match rtc_client.poll_loop().await? {
                RtcEvent::ChannelData { id, data } => {
                    if id == ORDERED_MSG_CHANNEL_ID {
                        if let Ok(msg) = PeerMessageBody::decode(&data[..]) {
                            let request_id = msg.request_id;
                            if let Some(response) = msg.response {
                                self.msg_channel().notify_response(request_id, response).await;
                            } else if let Some(request) = msg.request {
                                self.process_message_packet(request_id, request).await;
                            }
                        }
                    } else if id == UNORDERED_MSG_CHANNEL_ID {
                        if let Ok(msg) = PeerMessageBody::decode(&data[..]) {
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
                RtcEvent::IceConnectionStateChange(state) => {
                    log::info!("[webrtc-client] ICE state: {:?}", state);
                }
                RtcEvent::Closed => {
                    return Ok(());
                }
                _ => {}
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
        let mut fec_sender = FecSender::new(1024);
        let mut last_peer_block_id = 0u32;
        const WINDOW_SIZE: u32 = 128;
        const MAX_BUFFER_SIZE: usize = 1024 * 1024 * 5;
        const MIN_BUFFER_SIZE: usize = CHUNK_SIZE;
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
                buff_counter = MIN_BUFFER_SIZE;

                if let Ok(hold_delimiter) = TransferDelimiterShema::hold(1).as_bytes() {
                    if let Ok(FecAction::Framed(frames)) = fec_sender.send(0, hold_delimiter.to_vec().into_boxed_slice()) {
                        for frame in frames {
                            let _ = reliable_tx.send(frame.serialize()).await;
                        }
                    }
                }
            }
        }
    }
}
