use crate::app::operations::p2p::{P2POperationOutput, P2PSessionOverview};
use crate::app::operations::CoreOperationOutput;
use crate::entities::local_resource::{LocalResource, LocalResourcePath, ResourceType};
use crate::entities::peer::Peer as PeerEntity;
use crate::entities::transfer_session::{TransferProgress, TransferSession};
use crate::protocol::webrtc::errors::WebRtcErrors;
use crate::protocol::webrtc::fec::{FecAction, FecReceiver, FecSender, Frame, CHUNK_SIZE};
use crate::protocol::webrtc::message_channel::DirectMessageChannel;
use crate::protocol::webrtc::transfer::{TransferDelimiterShema, TransfersContext};
use crate::protocol::webrtc::webrtc::{
    MAX_BUFFER_SIZE, MIN_BUFFER_SIZE, TRANSFER_RESOURCE2_UNRELIABLE_CHANNEL_ID, TRANSFER_RESOURCE3_UNRELIABLE_CHANNEL_ID,
    TRANSFER_RESOURCE4_UNRELIABLE_CHANNEL_ID, TRANSFER_RESOURCE_RELIABLE_CHANNEL_ID, TRANSFER_RESOURCE_UNRELIABLE_CHANNEL_ID,
};
use crate::repository::local_resource::LocalResourceRepository;
use crate::shell::api::{BufferExt, CoreRequest};
use crate::utils::compression::is_compressible;
use anyhow::anyhow;
use bytes::Bytes;
use core_services::utils::yield_container::YieldContainer;
use futures::channel::mpsc;
use futures::channel::mpsc::unbounded;
use futures_util::lock::Mutex;
use futures_util::{select, FutureExt, StreamExt};
use futures_util::{select_biased, SinkExt};
use matchbox_protocol::PeerId;
use matchbox_socket::{Packet, PeerBuffered};
use n0_future::task::spawn;
use n0_future::time::{sleep, Instant};
use once_cell::sync::OnceCell;
use schema::devlog::bitbridge::fec_feedback::Feedback;
use schema::devlog::bitbridge::peer_message_body::Response::IntroduceResponse;
use schema::devlog::bitbridge::peer_message_body::{Request, Response};
use schema::devlog::bitbridge::{CancelTransferSessionRequest, DownloadResourceRequest, IntroduceRequestMessage, IntroduceResponseMessage, P2pSessionOverviewMessage, P2pTransferSessionMessage, PeerErrorsMessage, PeerMessage, ResourceTypeMessage, SessionsNotificationMessage, ViewSessionDetailRequest, ViewSessionDetailResponse};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU16, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use core_services::utils::cancellation::CancellationToken;
use crate::app::operations::transfer::TransferOperationOutput;

// Global atomic counter for generating unique transfer IDs
static TRANSFER_ID_COUNTER: AtomicU16 = AtomicU16::new(1);

/// Quad-channel wrapper for load balancing between four unreliable data channels
pub struct QuadUnreliableChannel {
    channels: [mpsc::UnboundedSender<(PeerId, Packet)>; 4],
    channel_ids: [usize; 4],
    buffer: PeerBuffered,
    current_channel: u8,
}

impl QuadUnreliableChannel {
    pub fn new(
        channel1: mpsc::UnboundedSender<(PeerId, Packet)>,
        channel2: mpsc::UnboundedSender<(PeerId, Packet)>,
        channel3: mpsc::UnboundedSender<(PeerId, Packet)>,
        channel4: mpsc::UnboundedSender<(PeerId, Packet)>,
        channel1_id: usize,
        channel2_id: usize,
        channel3_id: usize,
        channel4_id: usize,
        buffer: PeerBuffered,
    ) -> Self {
        Self {
            channels: [
                channel1, channel2, channel3, channel4,
            ],
            channel_ids: [
                channel1_id,
                channel2_id,
                channel3_id,
                channel4_id,
            ],
            buffer,
            current_channel: 0,
        }
    }

    /// Send a packet, load balancing between the four channels
    pub fn send(&mut self, peer_id: PeerId, packet: Packet) -> Result<(), mpsc::TrySendError<(PeerId, Packet)>> {
        let channel_index = self.current_channel as usize;
        let result = self.channels[channel_index].unbounded_send((peer_id, packet));
        self.current_channel = (self.current_channel + 1) % 4;
        result
    }

    /// Wait for all four channels to have low buffer usage
    pub async fn wait_buffer_low(&self, min_buffer_size: usize, timeout: Duration) {
        for &channel_id in &self.channel_ids {
            self.buffer.wait_buffer_low(channel_id, min_buffer_size, timeout).await;
        }
    }

    /// Get combined bytes sent/received stats from all four channels
    pub async fn bytes_sent_received(&self) -> (usize, usize) {
        let mut total_sent = 0;
        let mut total_received = 0;

        for &channel_id in &self.channel_ids {
            let (sent, received) = self.buffer.channel_bytes_sent_received(channel_id).await.unwrap_or((0, 0));
            total_sent += sent;
            total_received += received;
        }

        (total_sent, total_received)
    }

    /// Get bytes sent from all four channels
    pub async fn bytes_sent(&self) -> usize {
        let mut total_sent = 0;

        for &channel_id in &self.channel_ids {
            let sent = self.buffer.channel_bytes_sent_received(channel_id).await.map(|it| it.0).unwrap_or(0);
            total_sent += sent;
        }

        total_sent
    }

    /// Flush all four channels with timeout
    pub async fn flush_timeout(&self) -> Result<(), WebRtcErrors> {
        for &channel_id in &self.channel_ids {
            self.buffer.flush_timeout(channel_id).await?;
        }
        Ok(())
    }
}

pub struct WebRtcPeer {
    pub peer: PeerEntity,
    pub resource_repo: Arc<dyn LocalResourceRepository>,

    // Channel used to communicate with the peer
    pub msg_channel: DirectMessageChannel,
    // Quad unreliable channels for load-balanced data transfer
    pub quad_unreliable_channel: Arc<Mutex<QuadUnreliableChannel>>,
    pub reliable_data_channel: mpsc::UnboundedSender<(PeerId, Packet)>,
    // This channel is used to transfer the thumbnail
    pub thumbnail_channel: mpsc::UnboundedSender<(PeerId, Packet)>,
    // Webrtc buffer, used to control the amount of data that can be sent to the peer
    pub buffer: PeerBuffered,

    pub transfer_feedback_receiver: YieldContainer<mpsc::UnboundedReceiver<Feedback>>,
    pub transfer_feedback_sender: mpsc::UnboundedSender<Feedback>,

    pub transfers_context: TransfersContext,

    pub inbound_thumbnail_stream_receiver: YieldContainer<mpsc::Receiver<Packet>>,
    pub inbound_thumbnail_stream_sender: mpsc::Sender<Packet>,
    pub inbound_data_stream_receiver: YieldContainer<mpsc::Receiver<Packet>>,
    pub inbound_data_stream_sender: mpsc::Sender<Packet>,

    pub outbound_packet_receiver: YieldContainer<mpsc::Receiver<(u16, Packet)>>,
    pub outbound_packet_sender: mpsc::Sender<(u16, Packet)>,

    pub prefix_channels: Arc<Mutex<HashMap<u16, mpsc::UnboundedSender<Packet>>>>,

    pub bandwidth: Arc<AtomicU64>,

    // Connect to the core stream, where all state is stored
    pub core_request: OnceCell<CoreRequest>,
}

impl WebRtcPeer {
    pub async fn new(
        user: PeerEntity,
        msg_channel: DirectMessageChannel,
        reliable_data_channel: mpsc::UnboundedSender<(PeerId, Packet)>,
        unreliable_data_channel: mpsc::UnboundedSender<(PeerId, Packet)>,
        unreliable2_data_channel: mpsc::UnboundedSender<(PeerId, Packet)>,
        unreliable3_data_channel: mpsc::UnboundedSender<(PeerId, Packet)>,
        unreliable4_data_channel: mpsc::UnboundedSender<(PeerId, Packet)>,
        thumbnail_channel: mpsc::UnboundedSender<(PeerId, Packet)>,
        buffer: PeerBuffered,
        repository: Arc<dyn LocalResourceRepository>,
    ) -> Result<Self, WebRtcErrors> {
        let (transfer_feedback_sender, transfer_feedback_receiver) = unbounded();

        let (thumbnail_data_tx, thumbnail_data_rx) = mpsc::channel(1024);
        let (data_tx, data_rx) = mpsc::channel(1024);
        let (outbound_packet_tx, outbound_packet_rx) = mpsc::channel(16);

        let introduce_request = IntroduceRequestMessage {
            mine: PeerMessage {
                peer_id: user.id().to_string(),
                name: user.name.clone(),
                avatar_url: user.avatar_url.clone(),
                device: user.device.clone().into(),
                email: user.email.clone(),
            },
        };

        log::info!("Sending introduce request to other peer {:?}", introduce_request.mine.peer_id);
        let IntroduceResponse(response) = msg_channel.send(Request::IntroduceRequest(introduce_request), None).await? else {
            return Err(WebRtcErrors::FailedToIntroducePeer);
        };

        log::info!("Received introduce response from other peer {:?}", response.peer.peer_id);

        let peer: PeerEntity = response.peer.into();

        let quad_unreliable_channel = Arc::new(Mutex::new(QuadUnreliableChannel::new(
            unreliable_data_channel,
            unreliable2_data_channel,
            unreliable3_data_channel,
            unreliable4_data_channel,
            TRANSFER_RESOURCE_UNRELIABLE_CHANNEL_ID,
            TRANSFER_RESOURCE2_UNRELIABLE_CHANNEL_ID,
            TRANSFER_RESOURCE3_UNRELIABLE_CHANNEL_ID,
            TRANSFER_RESOURCE4_UNRELIABLE_CHANNEL_ID,
            buffer.clone(),
        )));

        Ok(Self {
            msg_channel,
            peer,
            transfer_feedback_receiver: YieldContainer::new(transfer_feedback_receiver),
            transfer_feedback_sender,
            reliable_data_channel,
            quad_unreliable_channel,
            thumbnail_channel,
            transfers_context: TransfersContext::new(),
            resource_repo: repository,
            inbound_thumbnail_stream_sender: thumbnail_data_tx,
            inbound_data_stream_sender: data_tx,
            inbound_data_stream_receiver: YieldContainer::new(data_rx),
            inbound_thumbnail_stream_receiver: YieldContainer::new(thumbnail_data_rx),
            outbound_packet_receiver: YieldContainer::new(outbound_packet_rx),
            outbound_packet_sender: outbound_packet_tx,
            prefix_channels: Arc::new(Mutex::new(HashMap::new())),
            bandwidth: Arc::new(AtomicU64::new(0)),
            buffer,
            core_request: Default::default(),
        })
    }

    pub fn core_request(&self) -> Option<&CoreRequest> {
        self.core_request.get()
    }

    pub async fn from_introduce_request(
        user: PeerEntity,
        request_id: String,
        msg: IntroduceRequestMessage,
        msg_channel: DirectMessageChannel,
        reliable_data_channel: mpsc::UnboundedSender<(PeerId, Packet)>,
        unreliable_data_channel: mpsc::UnboundedSender<(PeerId, Packet)>,
        unreliable2_data_channel: mpsc::UnboundedSender<(PeerId, Packet)>,
        unreliable3_data_channel: mpsc::UnboundedSender<(PeerId, Packet)>,
        unreliable4_data_channel: mpsc::UnboundedSender<(PeerId, Packet)>,
        thumbnail_channel: mpsc::UnboundedSender<(PeerId, Packet)>,
        buffer: PeerBuffered,
        repository: Arc<dyn LocalResourceRepository>,
    ) -> Result<Self, WebRtcErrors> {
        log::info!("Received introduce request from other peer {:?}", msg.mine.peer_id);
        let (transfer_feedback_sender, transfer_feedback_receiver) = unbounded();
        let (thumbnail_data_tx, thumbnail_data_rx) = mpsc::channel(1024);
        let (data_tx, data_rx) = mpsc::channel(1024);
        let (outbound_packet_tx, outbound_packet_rx) = mpsc::channel(16);
        let introduce_response = IntroduceResponse(IntroduceResponseMessage {
            peer: PeerMessage {
                peer_id: user.id().to_string(),
                name: user.name.clone(),
                avatar_url: user.avatar_url.clone(),
                device: user.device.clone().into(),
                email: user.email.clone(),
            },
        });

        msg_channel.send_response(request_id, introduce_response).await?;
        log::info!("Sent introduce response to other peer {:?}", msg.mine.peer_id);

        let quad_unreliable_channel = Arc::new(Mutex::new(QuadUnreliableChannel::new(
            unreliable_data_channel,
            unreliable2_data_channel,
            unreliable3_data_channel,
            unreliable4_data_channel,
            TRANSFER_RESOURCE_UNRELIABLE_CHANNEL_ID,
            TRANSFER_RESOURCE2_UNRELIABLE_CHANNEL_ID,
            TRANSFER_RESOURCE3_UNRELIABLE_CHANNEL_ID,
            TRANSFER_RESOURCE4_UNRELIABLE_CHANNEL_ID,
            buffer.clone(),
        )));

        Ok(Self {
            msg_channel,
            transfer_feedback_sender,
            transfer_feedback_receiver: YieldContainer::new(transfer_feedback_receiver),
            peer: msg.mine.into(),
            reliable_data_channel,
            quad_unreliable_channel,
            thumbnail_channel,
            transfers_context: TransfersContext::new(),
            resource_repo: repository,
            inbound_thumbnail_stream_sender: thumbnail_data_tx,
            inbound_data_stream_sender: data_tx,
            inbound_data_stream_receiver: YieldContainer::new(data_rx),
            inbound_thumbnail_stream_receiver: YieldContainer::new(thumbnail_data_rx),
            outbound_packet_receiver: YieldContainer::new(outbound_packet_rx),
            outbound_packet_sender: outbound_packet_tx,
            prefix_channels: Arc::new(Mutex::new(HashMap::new())),
            bandwidth: Arc::new(AtomicU64::new(0)),
            buffer,
            core_request: Default::default(),
        })
    }

    pub fn start_core_stream(&self, core_request: CoreRequest) {
        let _ = self.core_request.set(core_request);
    }

    pub async fn process_message_packet(&self, request_id: String, msg: Request) {
        match msg {
            Request::CancelRequest(request) => {
                self.transfers_context.cancel_transfer(request.session_id as u64).await;
            }
            Request::FecFeedback(feedback) => {
                if let Some(feedback) = feedback.feedback {
                    log::info!("Received FEC feedback: {:?}", feedback);
                    let _ = self.transfer_feedback_sender.unbounded_send(feedback);
                };
            }
            Request::SessionsNotification(notification) => {
                let sessions: Vec<P2PSessionOverview> = notification.sessions.iter().map(|s| {
                    P2PSessionOverview {
                        order_id: s.order_id,
                        password_protected: s.password_protected,
                    }
                }).collect();
                let response = CoreOperationOutput::P2P(P2POperationOutput::ReceivedSessionsOverview {
                    peer_id: self.peer.id().to_string(),
                    sessions,
                });
                if let Some(core_request) = self.core_request() {
                    core_request.response(response).await;
                }
            }
            Request::ViewSessionRequest(req) => {
                let response = CoreOperationOutput::P2P(P2POperationOutput::ReceivedViewSessionRequest {
                    peer_id: self.peer.id().to_string(),
                    request_id,
                    order_id: req.order_id,
                    password: req.password,
                });
                if let Some(core_request) = self.core_request() {
                    core_request.response(response).await;
                }
            }
            Request::DownloadResourceRequest(req) => {
                let response = CoreOperationOutput::P2P(P2POperationOutput::ReceivedDownloadRequest {
                    peer_id: self.peer.id().to_string(),
                    session_order_id: req.session_order_id,
                    resource_order_id: req.resource_order_id,
                    transfer_id: req.transfer_id as u16,
                });
                if let Some(core_request) = self.core_request() {
                    core_request.response(response).await;
                }
            }
            _ => {}
        }
    }

    pub async fn process_data_packet(&self, packet: Packet) {
        let _ = self.inbound_data_stream_sender.clone().try_send(packet);
    }

    pub async fn process_thumbnail_packet(&self, packet: Packet) {
        let _ = self.inbound_thumbnail_stream_sender.clone().try_send(packet);
    }

    pub async fn peer_disconnected(&self) {
        log::info!("Peer disconnected, will cancel all transfers");
        self.transfers_context.cancel_all_transfers().await;
        let response = CoreOperationOutput::P2P(P2POperationOutput::PeerDisconnected {});
        if let Some(core_request) = self.core_request() {
            core_request.response(response).await;
        }
    }

    pub async fn cancel_transfer(&self, session_id: u64) {
        let cancel_msg = CancelTransferSessionRequest {
            session_id: session_id as i64,
        };

        self.transfers_context.cancel_transfer(session_id).await;

        log::info!("Cancelling transfer session {session_id} to peer {}", self.peer.peer_id());
        let request = Request::CancelRequest(cancel_msg);
        let _ = self.msg_channel.notify(request).await;
    }

    async fn handle_fec_action(&self, action: FecAction) -> Result<(Vec<Packet>, Option<Instant>), WebRtcErrors> {
        match action {
            FecAction::Constructed(packets_with_prefix, next_check) => {
                let packets = packets_with_prefix.into_iter().map(|(_, packet)| packet).collect();
                Ok((packets, Some(next_check)))
            },
            FecAction::Feedback(fb, next_check) => {
                log::info!("Sending FEC feedback: {:?}", fb);
                self.msg_channel.notify(Request::FecFeedback(fb)).await?;
                Ok((vec![], Some(next_check)))
            }
            FecAction::Terminated => {
                log::warn!("FEC terminated");
                Err(WebRtcErrors::InvalidDelimiter("FEC terminated".into()))
            }
            FecAction::Queued(time) => Ok((vec![], Some(time))),
            FecAction::Noop => Ok((vec![], None)),
            _ => Ok((vec![], None)),
        }
    }

    pub async fn cancel_transfer_session(&self, session_id: u64) -> Result<(), WebRtcErrors> {
        self.transfers_context.cancel_transfer(session_id).await;
        self.msg_channel
            .notify(Request::CancelRequest(CancelTransferSessionRequest {
                session_id: session_id as i64,
            }))
            .await?;
        Ok(())
    }

    pub async fn sending_loop(&self) -> Result<(), WebRtcErrors> {
        let mut fec_sender = FecSender::new(self.peer.peer_id(), 512);
        let mut feedback_receiver = self.transfer_feedback_receiver.retrieve().await?;
        let mut packet_rx = self.outbound_packet_receiver.retrieve().await?;
        let mut buff_counter = 0;

        loop {
            if let Some(rtt) = self.buffer.rtt().await {
                fec_sender.set_rtt(rtt as u64);
            }

            let packet_fut = packet_rx.next().fuse();
            let fb_fut = feedback_receiver.next().fuse();

            futures::pin_mut!(packet_fut);
            futures::pin_mut!(fb_fut);

            let action = select_biased! {
                pkt = packet_fut => {
                    match pkt {
                        Some((prefix, packet)) => fec_sender.send(prefix, packet)?,
                        None => break,
                    }
                },
                fb = fb_fut => {
                    match fb {
                        Some(fb) => fec_sender.feedback(fb),
                        _ => FecAction::Noop,
                    }
                },
            };

            match action {
                FecAction::Framed(frames) => {
                    let mut quad_channel = self.quad_unreliable_channel.lock().await;
                    for frame in frames {
                        let packet = frame.serialize();
                        buff_counter += packet.len();
                        let _ = quad_channel.send(self.peer.peer_id(), packet);
                    }
                },
                FecAction::Retransmit(frames) => {
                    for frame in frames {
                        let packet = frame.serialize();
                        buff_counter += packet.len();
                        let _ = self.reliable_data_channel.unbounded_send((self.peer.peer_id(), packet));
                    }
                },
                FecAction::Terminated => {
                    log::info!("FEC sender terminated in sending_loop");
                    break;
                },
                _ => {}
            }

            if buff_counter > MAX_BUFFER_SIZE {
                buff_counter = 0;

                let tick = Instant::now();
                let quad_ch = self.quad_unreliable_channel.lock().await;
                let stats_before = quad_ch.bytes_sent().await;

                quad_ch.wait_buffer_low(MIN_BUFFER_SIZE, Duration::from_millis(1500)).await;
                self.buffer
                    .wait_buffer_low(
                        TRANSFER_RESOURCE_RELIABLE_CHANNEL_ID,
                        MIN_BUFFER_SIZE,
                        Duration::from_millis(4 * fec_sender.rtt().max(MIN_BUFFER_SIZE as u64)),
                    )
                    .await;

                let hold_delimiter = TransferDelimiterShema::hold().as_bytes()?;
                let FecAction::Framed(frames) = fec_sender.send(0, hold_delimiter)? else {
                    return Err(anyhow!("Failed to build hold delimiter").into());
                };

                for frame in frames {
                    let _ = self.reliable_data_channel.unbounded_send((self.peer.peer_id(), frame.serialize()));
                }

                let time = tick.elapsed().as_secs_f64().max(f64::MIN);
                let stats_after = quad_ch.bytes_sent().await;
                let total_sent = stats_after.saturating_sub(stats_before);
                let bw = (total_sent as f64 / time) as u64;
                let bw_kbps = bw / 1000;

                if bw_kbps > 0 {
                    self.bandwidth.store(bw_kbps, Ordering::Relaxed);
                    log::info!(
                        "Buffer low, sent {} bytes in {} seconds, bandwidth: {} kbps",
                        total_sent,
                        time,
                        bw_kbps
                    );
                }
            }
        }

        Ok(())
    }

    pub async fn send_sessions_notification(
        &self,
        sessions: Vec<TransferSession>,
    ) -> Result<(), WebRtcErrors> {
        let overviews: Vec<P2pSessionOverviewMessage> = sessions
            .iter()
            .map(|session| {
                let password_protected = matches!(&session.target, crate::entities::target::TransferTarget::P2P { password: Some(_), .. });
                P2pSessionOverviewMessage {
                    order_id: session.order_id,
                    password_protected,
                }
            })
            .collect();

        let notification = SessionsNotificationMessage { sessions: overviews };
        let request = Request::SessionsNotification(notification);
        self.msg_channel.notify(request).await?;
        Ok(())
    }

    pub async fn request_session_detail(
        &self,
        core_request: CoreRequest,
        order_id: u64,
        password: Option<String>,
    ) -> Result<(), WebRtcErrors> {
        use schema::devlog::bitbridge::view_session_detail_response::Result as ResponseResult;
        let request = ViewSessionDetailRequest {
            order_id,
            password,
        };

        let mut response = Box::pin(self.msg_channel.stream(Request::ViewSessionRequest(request)).await?);
        while let Some(Response::ViewSessionResponse(resp)) = response.next().await {
            match resp.result {
                Some(ResponseResult::Session(proto_session)) => {
                    let session = TransferSession {
                        order_id: proto_session.order_id,
                        resources: vec![],
                        progress: vec![],
                        transfer_type: crate::entities::transfer_session::TransferType::Receive,
                        target: crate::entities::target::TransferTarget::P2P {
                            from_peer: self.peer.clone(),
                            password: None,
                            is_required_password: false,
                        },
                        cancellation_token: CancellationToken::new(),
                    };

                    core_request.response(session).await;
                }
                Some(ResponseResult::ResourceUpdated(resource_proto)) => {
                    let mut resource = LocalResource {
                        order_id: resource_proto.order_id,
                        name: resource_proto.name,
                        size: resource_proto.size as u64,
                        path: LocalResourcePath::RelativePath {
                            path: format!("received/session_{}/resource_{}", order_id, resource_proto.order_id),
                            is_private: false,
                        },
                        thumbnail_path: None,
                        r#type: (ResourceTypeMessage::try_from(resource_proto.r#type).unwrap_or_default()).try_into().unwrap_or(ResourceType::File),
                    };

                    if let Some(thumbnail_bytes) = resource_proto.thumbnail_png {
                        match self.resource_repo.save_thumbnail(thumbnail_bytes, resource.order_id).await {
                            Ok(thumbnail_path) => {
                                resource.thumbnail_path = Some(thumbnail_path);
                            }
                            Err(e) => {
                                log::warn!("Failed to save thumbnail for resource {}: {:?}", resource.order_id, e);
                            }
                        }
                    }

                    core_request.response(resource).await;
                }
                Some(ResponseResult::Error(error_type)) => {
                    let error_msg = PeerErrorsMessage::try_from(error_type)
                        .unwrap_or(PeerErrorsMessage::InvalidRequest);
                    return Err(WebRtcErrors::PeerError(error_msg.to_string()));
                }
                None => break,
            }
        }

        Ok(())
    }

    pub async fn send_session_detail_response(
        &self,
        request_id: String,
        session: Option<&TransferSession>,
        error: Option<String>,
    ) -> Result<(), WebRtcErrors> {
        use schema::devlog::bitbridge::view_session_detail_response::Result as ResponseResult;

        if let Some(error_msg) = error {
            let error_type = if error_msg.contains("password") {
                PeerErrorsMessage::InvalidPassword
            } else if error_msg.contains("not found") {
                PeerErrorsMessage::SessionNotFound
            } else {
                PeerErrorsMessage::InvalidRequest
            };

            self.msg_channel.send_response(request_id, Response::ViewSessionResponse(ViewSessionDetailResponse { result: Some(ResponseResult::Error(error_type.into())) })).await?;
            return Ok(())
        }

        let Some(session) = session else {
            return Ok(())
        };

        let proto_session = P2pTransferSessionMessage {
            order_id: session.order_id,
            resources: vec![],
        };

        let response = ViewSessionDetailResponse {
            result: Some(ResponseResult::Session(proto_session))
        };

        self.msg_channel.send_response(request_id.clone(), Response::ViewSessionResponse(response)).await?;
        for resource in &session.resources {
            let mut resource_proto = resource.to_proto();
            if let Some(thumbnail_path) = resource.thumbnail_path.as_ref() {
                let Ok(mut thumbnail_cursor) = self.resource_repo.read(thumbnail_path.clone(), 64 * 1024, false).await else {
                    continue;
                };

                let Ok(bytes) = thumbnail_cursor.read_all().await else {
                    continue;
                };

                resource_proto.thumbnail_png = Some(bytes.to_vec());

                self.msg_channel.send_response(request_id.clone(), Response::ViewSessionResponse(ViewSessionDetailResponse { result: Some(ResponseResult::ResourceUpdated(resource_proto)) })).await?;
            }
        }

        Ok(())
    }

    pub async fn request_resource_download(
        &self,
        core_request: CoreRequest,
        session_order_id: u64,
        resource: LocalResource,
        mut progress: TransferProgress,
    ) -> Result<(), WebRtcErrors> {
        let resource_order_id = resource.order_id;

        let transfer_id = TRANSFER_ID_COUNTER.fetch_add(1, Ordering::Relaxed);

        let request = DownloadResourceRequest {
            session_order_id,
            resource_order_id,
            transfer_id: transfer_id as u32,
        };

        let (tx, mut rx) = mpsc::unbounded::<Packet>();

        let prefix = transfer_id;
        {
            let mut channels = self.prefix_channels.lock().await;
            channels.insert(prefix, tx);
        }

        self.msg_channel.notify(Request::DownloadResourceRequest(request)).await?;
        let resource_repo = self.resource_repo.clone();
        let prefix_channels = self.prefix_channels.clone();

        let start_delimiter = loop {
            if let Some(packet) = rx.next().await {
                if let Ok(delimiter) = TransferDelimiterShema::from_start_packet(&packet, session_order_id) {
                    break delimiter;
                }
            } else {
                log::warn!("Channel closed before receiving start delimiter");
                return Err(WebRtcErrors::InvalidDelimiter("Channel closed before start delimiter".into()));
            }
        };

        let Some(resource_id) = start_delimiter.resource_id() else {
            log::error!("Start delimiter missing resource_id");
            return Err(WebRtcErrors::InvalidDelimiter("Start delimiter missing resource_id".into()));
        };

        let compressed = start_delimiter.compressed();

        let mut writer = match resource_repo.write(resource.path.clone(), compressed).await {
            Ok(w) => w,
            Err(e) => {
                log::error!("Failed to create writer: {:?}", e);
                return Err(WebRtcErrors::InvalidDelimiter(format!("Failed to create writer: {:?}", e)));
            }
        };

        loop {
            let Some(packet) = rx.next().await else {
                log::warn!("Channel closed before receiving end delimiter");
                break;
            };

            if TransferDelimiterShema::from_end_packet(&packet, session_order_id).is_ok() {
                log::info!("Received end delimiter for resource {}", resource_id);
                progress.success();
                core_request.response(TransferOperationOutput::TransferResourceProgressUpdate(progress.clone())).await;
                break;
            }

            if TransferDelimiterShema::from_hold_packet(&packet).is_ok() {
                continue;
            }

            let bytes = Bytes::from(packet.to_vec());
            let written = writer.write(bytes).await?;
            progress.update_progress(written as u64);
            core_request.response_throttle(TransferOperationOutput::TransferResourceProgressUpdate(progress.clone())).await;
        }

        prefix_channels.lock().await.remove(&prefix);

        log::info!("Completed download for resource {}", resource_id);

        Ok(())
    }

    pub async fn stream_resource(
        &self,
        session_id: u64,
        transfer_id: u16,
        resource: LocalResource,
    ) -> Result<(), WebRtcErrors> {
        let resource_id = resource.order_id;
        let prefix = transfer_id;

        let resource_name = match resource.r#type {
            ResourceType::Folder => format!("{}.zip", &resource.name),
            _ => resource.name.clone(),
        };

        let compressed = is_compressible(&resource_name);

        let start_delimiter = TransferDelimiterShema::start(session_id, resource_id, compressed);
        let start_packet = start_delimiter.as_bytes()?;
        let mut outbound_packet_sender = self.outbound_packet_sender.clone();
        outbound_packet_sender.send((prefix, start_packet)).await
            .map_err(|e| WebRtcErrors::InvalidDelimiter(format!("Failed to send start delimiter: {:?}", e)))?;

        let mut cursor = self.resource_repo.read(resource.path.clone(), CHUNK_SIZE, compressed).await?;

        loop {
            cursor.compression_stats_mut().update_network_bandwidth(self.bandwidth.load(Ordering::Relaxed) as f64 * 1024f64);
            match cursor.c_next(Some(CHUNK_SIZE as u64)).await {
                Ok(Some((data, _raw_size))) => {
                    if data.is_empty() {
                        break;
                    }

                    let packet = data.to_vec().into_boxed_slice();
                    self.outbound_packet_sender.clone().send((prefix, packet)).await
                        .map_err(|e| anyhow!("Failed to send data packet: {:?}", e))?;
                }
                Ok(None) => {
                    break;
                }
                Err(e) => {
                    log::error!("Error reading resource data: {:?}", e);
                    return Err(anyhow!("Failed to read resource: {:?}", e).into());
                }
            }
        }

        let end_delimiter = TransferDelimiterShema::end(session_id, resource_id, compressed);
        let end_packet = end_delimiter.as_bytes()?;
        outbound_packet_sender.send((prefix, end_packet)).await
            .map_err(|e| WebRtcErrors::InvalidDelimiter(format!("Failed to send end delimiter: {:?}", e)))?;

        log::info!("Completed streaming resource {} for session {}", resource_id, session_id);
        Ok(())
    }

    pub async fn receiving_loop(&self) -> Result<(), WebRtcErrors> {
        let mut fec_receiver = FecReceiver::new();
        let mut data_rx = self.inbound_data_stream_receiver.retrieve().await?;

        loop {
            let frames = {
                let mut frames = Vec::new();

                if let Some(packet) = data_rx.next().await {
                    if let Some(frame) = Frame::deserialize(&packet) {
                        frames.push(frame);
                    }
                }

                while let Some(packet) = data_rx.try_next().ok().flatten() {
                    if let Some(frame) = Frame::deserialize(&packet) {
                        frames.push(frame);
                    }
                }

                frames
            };

            if let Some(rtt) = self.buffer.rtt().await {
                fec_receiver.set_rtt(rtt as u64);
            }

            let action = fec_receiver.receive(frames)?;

            match action {
                FecAction::Constructed(packets_with_prefix, _next_check) => {
                    for (prefix, packet) in packets_with_prefix {
                        let channels = self.prefix_channels.lock().await;
                        if let Some(sender) = channels.get(&prefix) {
                            let mut sender_clone = sender.clone();
                            if let Err(e) = sender_clone.send(packet).await {
                                log::warn!("Failed to send packet to prefix {} channel: {:?}", prefix, e);
                            }
                        } else {
                            log::warn!("No channel registered for prefix {}", prefix);
                        }
                    }
                },
                FecAction::Feedback(fb, _next_check) => {
                    log::info!("Sending FEC feedback from receiver: {:?}", fb);
                    let _ = self.msg_channel.notify(Request::FecFeedback(fb)).await;
                },
                FecAction::Terminated => {
                    log::warn!("FEC receiver terminated");
                    break;
                },
                _ => {}
            }
        }

        Ok(())
    }

    pub async fn run_loop(&self) -> Result<(), WebRtcErrors> {
        let mut send_fut = self.sending_loop().fuse();
        let mut recv_fut = self.receiving_loop().fuse();
        futures::pin_mut!(send_fut);
        futures::pin_mut!(recv_fut);
        select! {
            r = send_fut => r,
            r = recv_fut => r,
        }
    }
}

impl Drop for WebRtcPeer {
    fn drop(&mut self) {
        log::info!("Dropped peer {:?}", self.peer.peer_id());
    }
}
