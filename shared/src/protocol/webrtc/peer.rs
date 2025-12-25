use crate::app::operations::p2p::{P2POperationOutput, P2PSessionOverview};
use crate::app::operations::transfer::TransferOperationOutput;
use crate::app::operations::transfer::TransferOperationOutput::TransferResourceProgressUpdate;
use crate::app::operations::CoreOperationOutput;
use crate::entities::local_resource::{LocalResource, LocalResourcePath, ResourceType};
use crate::entities::peer::Peer as PeerEntity;
use crate::entities::transfer_session::{ThumbnailUpdatedEvent, TransferSession, TransferSessionStatus};
use crate::protocol::webrtc::errors::WebRtcErrors;
use crate::protocol::webrtc::fec::{loss_delay_us, FecAction, FecReceiver, FecSender, Frame, CHUNK_SIZE, DATA_SHARDS_DEFAULT};
use crate::protocol::webrtc::message_channel::DirectMessageChannel;
use crate::protocol::webrtc::transfer::{TransferDelimiterShema, TransfersContext};
use crate::protocol::webrtc::webrtc::{
    MAX_BUFFER_SIZE, MIN_BUFFER_SIZE, TRANSFER_RESOURCE2_UNRELIABLE_CHANNEL_ID, TRANSFER_RESOURCE3_UNRELIABLE_CHANNEL_ID,
    TRANSFER_RESOURCE4_UNRELIABLE_CHANNEL_ID, TRANSFER_RESOURCE_RELIABLE_CHANNEL_ID, TRANSFER_RESOURCE_UNRELIABLE_CHANNEL_ID,
    TRANSFER_THUMBNAIL_CHANNEL_ID,
};
use crate::repository::errors::PersistenceError;
use crate::repository::local_resource::LocalResourceRepository;
use crate::shell::api::{BufferExt, CoreRequest};
use crate::utils::compression::is_compressible;
use anyhow::{anyhow, Context};
use core_services::utils::cancellation::{CancellationToken, FutureExtension};
use core_services::utils::yield_container::YieldContainer;
use futures::channel::mpsc;
use futures::channel::mpsc::unbounded;
use futures_util::lock::Mutex;
use futures_util::FutureExt;
use futures_util::{select_biased, SinkExt};
use matchbox_protocol::PeerId;
use matchbox_socket::{Packet, PeerBuffered};
use n0_future::task::spawn;
use n0_future::time::{sleep, Instant};
use n0_future::StreamExt;
use once_cell::sync::OnceCell;
use schema::devlog::bitbridge::fec_feedback::Feedback;
use schema::devlog::bitbridge::peer_message_body::Response::IntroduceResponse;
use schema::devlog::bitbridge::peer_message_body::{Request, Response};
use schema::devlog::bitbridge::{
    CancelTransferSessionRequest, FecFeedback, IntroduceRequestMessage, IntroduceResponseMessage, PeerMessage,
};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

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

    pub outbound_packet_receiver: YieldContainer<mpsc::Receiver<(u8, Packet)>>,
    pub outbound_packet_sender: mpsc::Sender<(u8, Packet)>,

    pub prefix_channels: Arc<Mutex<HashMap<u8, mpsc::Sender<Packet>>>>,

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
        todo!("Send SessionsNotificationMessage to peer")
    }

    pub async fn request_session_detail(
        &self,
        order_id: u64,
        password: Option<String>,
    ) -> Result<(), WebRtcErrors> {
        todo!("Send ViewSessionDetailRequest to peer")
    }

    pub async fn send_session_detail_response(
        &self,
        request_id: String,
        session: Option<&TransferSession>,
        error: Option<String>,
    ) -> Result<(), WebRtcErrors> {
        todo!("Send ViewSessionDetailResponse to peer")
    }

    pub async fn request_resource_download(
        &self,
        session_order_id: u64,
        resource_order_id: u64,
    ) -> Result<(), WebRtcErrors> {
        todo!("Send DownloadResourceRequest to peer")
    }

    pub async fn stream_resource(
        &self,
        resource: LocalResource,
    ) -> Result<(), WebRtcErrors> {
        todo!("Stream resource data to peer using existing transfer protocol")
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
                    let channels = self.prefix_channels.lock().await;

                    for (prefix, packet) in packets_with_prefix {
                        if let Some(sender) = channels.get(&prefix) {
                            let mut sender_clone = sender.clone();
                            if let Err(e) = sender_clone.try_send(packet) {
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
}

impl Drop for WebRtcPeer {
    fn drop(&mut self) {
        log::info!("Dropped peer {:?}", self.peer.peer_id());
    }
}
