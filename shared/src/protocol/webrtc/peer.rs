use crate::app::operations::p2p::P2POperationOutput;
use crate::app::operations::transfer::TransferOperationOutput;
use crate::app::operations::transfer::TransferOperationOutput::TransferResourceProgressUpdate;
use crate::app::operations::CoreOperationOutput;
use crate::entities::local_resource::{LocalResourcePath, ResourceType};
use crate::entities::peer::Peer as PeerEntity;
use crate::entities::transfer_session::{ThumbnailUpdatedEvent, TransferSession, TransferSessionStatus};
use crate::protocol::webrtc::errors::WebRtcErrors;
use crate::protocol::webrtc::fec::{FecAction, FecReceiver, FecSender, Frame, CHUNK_SIZE, DATA_SHARDS_DEFAULT};
use crate::protocol::webrtc::message_channel::DirectMessageChannel;
use crate::protocol::webrtc::transfer::{TransferDelimiterShema, TransfersContext};
use crate::protocol::webrtc::webrtc::{
    MAX_BUFFER_SIZE,
    MIN_BUFFER_SIZE,
    TRANSFER_RESOURCE_RELIABLE_CHANNEL_ID,
    TRANSFER_RESOURCE_UNRELIABLE_CHANNEL_ID,
    TRANSFER_RESOURCE2_UNRELIABLE_CHANNEL_ID,
    TRANSFER_RESOURCE3_UNRELIABLE_CHANNEL_ID,
    TRANSFER_RESOURCE4_UNRELIABLE_CHANNEL_ID,
    TRANSFER_THUMBNAIL_CHANNEL_ID
};
use crate::repository::errors::PersistenceError;
use crate::repository::local_resource::LocalResourceRepository;
use crate::shell::api::{BufferExt, CoreRequest};
use crate::utils::compression::is_compressible;
use anyhow::{anyhow, Context};
use core_services::utils::cancellation::FutureExtension;
use core_services::utils::yield_container::YieldContainer;
use futures::channel::mpsc;
use futures::channel::mpsc::unbounded;
use futures_util::lock::Mutex;
use futures_util::FutureExt;
use futures_util::{join, select, select_biased, SinkExt};
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
    BeginTransferResource,
    CancelTransferSessionRequest,
    EndTransferResource,
    FecFeedback,
    IntroduceRequestMessage,
    IntroduceResponseMessage,
    NetworkStats,
    PeerMessage,
    TransferRequestMessage,
    TransferResponseMessage,
    TransferSessionMessage
};
use std::sync::mpsc::channel;
use std::sync::Arc;
use std::time::Duration;

/// Quad-channel wrapper for load balancing between four unreliable data channels
pub struct QuadUnreliableChannel {
    channel1: mpsc::UnboundedSender<(PeerId, Packet)>,
    channel2: mpsc::UnboundedSender<(PeerId, Packet)>,
    channel3: mpsc::UnboundedSender<(PeerId, Packet)>,
    channel4: mpsc::UnboundedSender<(PeerId, Packet)>,
    channel1_id: usize,
    channel2_id: usize,
    channel3_id: usize,
    channel4_id: usize,
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
            channel1,
            channel2,
            channel3,
            channel4,
            channel1_id,
            channel2_id,
            channel3_id,
            channel4_id,
            buffer,
            current_channel: 0,
        }
    }

    /// Send a packet, load balancing between the four channels
    pub fn send(&mut self, peer_id: PeerId, packet: Packet) -> Result<(), mpsc::TrySendError<(PeerId, Packet)>> {
        let result = match self.current_channel {
            0 => self.channel1.unbounded_send((peer_id, packet)),
            1 => self.channel2.unbounded_send((peer_id, packet)),
            2 => self.channel3.unbounded_send((peer_id, packet)),
            _ => self.channel4.unbounded_send((peer_id, packet)),
        };

        self.current_channel = (self.current_channel + 1) % 4;
        result
    }

    /// Wait for all four channels to have low buffer usage
    pub async fn wait_buffer_low(&self, min_buffer_size: usize, timeout: Duration) {
        self.buffer.wait_buffer_low(self.channel1_id, min_buffer_size, timeout).await;
        self.buffer.wait_buffer_low(self.channel2_id, min_buffer_size, timeout).await;
        self.buffer.wait_buffer_low(self.channel3_id, min_buffer_size, timeout).await;
        self.buffer.wait_buffer_low(self.channel4_id, min_buffer_size, timeout).await;
    }

    /// Get combined bytes sent/received stats from all four channels
    pub async fn bytes_sent_received(&self) -> (usize, usize) {
        let stats1 = self.buffer
            .channel_bytes_sent_received(self.channel1_id)
            .await
            .unwrap_or((0, 0));
        let stats2 = self.buffer
            .channel_bytes_sent_received(self.channel2_id)
            .await
            .unwrap_or((0, 0));
        let stats3 = self.buffer
            .channel_bytes_sent_received(self.channel3_id)
            .await
            .unwrap_or((0, 0));
        let stats4 = self.buffer
            .channel_bytes_sent_received(self.channel4_id)
            .await
            .unwrap_or((0, 0));

        (stats1.0 + stats2.0 + stats3.0 + stats4.0, stats1.1 + stats2.1 + stats3.1 + stats4.1)
    }

    /// Get bytes sent from all four channels
    pub async fn bytes_sent(&self) -> usize {
        let sent1 = self.buffer
            .channel_bytes_sent_received(self.channel1_id)
            .await
            .map(|it| it.0)
            .unwrap_or(0);
        let sent2 = self.buffer
            .channel_bytes_sent_received(self.channel2_id)
            .await
            .map(|it| it.0)
            .unwrap_or(0);
        let sent3 = self.buffer
            .channel_bytes_sent_received(self.channel3_id)
            .await
            .map(|it| it.0)
            .unwrap_or(0);
        let sent4 = self.buffer
            .channel_bytes_sent_received(self.channel4_id)
            .await
            .map(|it| it.0)
            .unwrap_or(0);

        sent1 + sent2 + sent3 + sent4
    }

    /// Flush all four channels with timeout
    pub async fn flush_timeout(&self) -> Result<(), WebRtcErrors> {
        self.buffer.flush_timeout(self.channel1_id).await?;
        self.buffer.flush_timeout(self.channel2_id).await?;
        self.buffer.flush_timeout(self.channel3_id).await?;
        self.buffer.flush_timeout(self.channel4_id).await?;
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
            Request::TransferRequest(request) => {
                self.transfers_context.start_transfer(request.session.order_id, request_id).await;
                let response = CoreOperationOutput::P2P(P2POperationOutput::ReceivedSessionRequest {
                    remote_session: request.session,
                });

                if let Some(core_request) = self.core_request() {
                    core_request.response(response).await;
                }
            }
            Request::FecFeedback(feedback) => {
                if let Some(feedback) = feedback.feedback {
                    log::info!("Received FEC feedback: {:?}", feedback);
                    let _ = self.transfer_feedback_sender.unbounded_send(feedback);
                };
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

    pub async fn answer_transfer(
        &self,
        core_request: CoreRequest,
        session_id: u64,
        session: Option<TransferSession>,
    ) -> Result<TransferSessionStatus, WebRtcErrors> {
        let Some(mut session) = session else {
            // Denied
            if let Some(rtc_request_id) = self.transfers_context.rtc_request_id(session_id).await {
                let response = TransferResponseMessage {};
                self.msg_channel.send_response(rtc_request_id, Response::TransferResponse(response)).await?;
            };

            return Ok(TransferSessionStatus::Canceled);
        };

        let mut resource_rx = self.inbound_data_stream_receiver.retrieve_timed(Duration::from_secs(11)).await?;
        let _ = resource_rx.drain();

        let cancellation_signal = session.token().clone();
        self.transfers_context.add_token(session_id, cancellation_signal.clone()).await;
        let _drop_guard = cancellation_signal.drop_guard();

        log::info!(
            "Thumbnails info {:?}",
            session.resources.iter().map(|r| r.thumbnail_path.clone()).collect::<Vec<_>>()
        );

        let mut thumbnail_rx = self.inbound_thumbnail_stream_receiver.retrieve_timed(Duration::from_secs(11)).await?;

        let msg_channel = self.msg_channel.clone();
        let peer_id = session.peer().map(|it| it.peer_id()).context("This is not a peer session")?;
        let context = self.transfers_context.clone();
        let response = TransferResponseMessage {};
        if let Some(rtc_request_id) = context.rtc_request_id(session_id).await {
            if let Err(e) = msg_channel.send_response(rtc_request_id, Response::TransferResponse(response)).await {
                log::error!("Failed to send response to peer {peer_id}: {e:?}");
                cancellation_signal.cancel();
            }
        }

        let thumbnail_handle = {
            let mut thumbnail_paths = session
                .resources
                .iter()
                .filter_map(|r| r.thumbnail_path.clone().map(|it| (r.order_id, it)))
                .collect::<Vec<(u64, LocalResourcePath)>>();
            let repo = self.resource_repo.clone();
            let core_request = core_request.clone();
            let thumbnail_cancel_signal = cancellation_signal.child_token();
            spawn(async move {
                while !thumbnail_cancel_signal.is_cancelled() {
                    if thumbnail_paths.is_empty() {
                        return Ok(thumbnail_paths);
                    }

                    log::info!("Begin receiving thumbnail for session {session_id}");
                    let start_delimiter = TransferDelimiterShema::forward_to_next_resource(&mut thumbnail_rx, session_id)
                        .with_cancel(&thumbnail_cancel_signal)
                        .await??;

                    let Some(resource_index) = thumbnail_paths.iter().position(|it| it.0 == start_delimiter.resource_id()) else {
                        return Err(WebRtcErrors::InvalidDelimiter(format!(
                            "The first delimiter is not match with any resource {start_delimiter:?}"
                        )));
                    };

                    let resource_path = thumbnail_paths.swap_remove(resource_index).1;

                    let mut writer = repo
                        .write(resource_path.clone(), start_delimiter.compressed())
                        .with_cancel(&thumbnail_cancel_signal)
                        .await??;

                    while let Ok(Some(bytes)) = thumbnail_rx.next().with_cancel(&thumbnail_cancel_signal).await {
                        writer
                            .d_write(bytes.to_vec().into())
                            .await
                            .map_err(|it| WebRtcErrors::PersistentError(PersistenceError::IOError(format!("{it:?}"))))?;

                        if TransferDelimiterShema::from_end_packet(&bytes, session_id).is_ok() {
                            break;
                        }
                    }

                    writer.end().with_cancel(&thumbnail_cancel_signal).await??;

                    let event = ThumbnailUpdatedEvent {
                        resource_id: start_delimiter.resource_id(),
                        path: resource_path,
                    };

                    let _ = core_request.response(TransferOperationOutput::ThumbnailUpdated(event)).await;
                }

                Ok(thumbnail_paths)
            })
        };

        // Metrics tracking for averages
        let mut total_decode_time_us = 0u64;
        let mut total_decode_count = 0u64;
        let mut total_byte_received = 0u64;
        let mut total_write_time_us = 0u64;
        let mut total_write_count = 0u64;
        let mut total_written_bytes_all = 0u64;
        let mut network_stats = NetworkStats::default();

        loop {
            if session.is_completed() {
                log::warn!("Session {session_id} is completed");
                break;
            }

            let start_delim = loop {
                let timeout_fut = sleep(Duration::from_secs(10)).fuse();
                let packet_fut = resource_rx.next().with_cancel(&cancellation_signal).fuse();

                futures::pin_mut!(timeout_fut);
                futures::pin_mut!(packet_fut);

                select_biased! {
                    raw_packet = packet_fut => {
                        let Some(raw_packet) = raw_packet? else {
                            return Err(WebRtcErrors::InvalidDelimiter("Stream ended before start delimiter".into()));
                        };

                        // Try to parse as start delimiter
                        if let Ok(delimiter) = TransferDelimiterShema::from_start_packet(&raw_packet, session_id) {
                            log::info!("Received start delimiter for resource {}", delimiter.resource_id());
                            break delimiter;
                        }
                        // Ignore other packets while waiting for start delimiter
                        continue;
                    },
                    _ = timeout_fut => {
                        return Err(WebRtcErrors::InvalidDelimiter("Timeout waiting for start delimiter".into()));
                    },
                }
            };

            let _ = resource_rx.drain();

            {
                network_stats.current_block_id = None;
                let feedback = FecFeedback {
                    feedback: Some(Feedback::Network(network_stats.clone())),
                };

                log::info!("Sending initial network stats for resource {}", start_delim.resource_id());
                self.msg_channel.notify(Request::FecFeedback(feedback)).await?;
            }

            // Create new FecReceiver for this resource
            let mut fec_receiver = FecReceiver::new();
            let mut next_check_time: Option<Instant> = None;

            let Some((resource_path, resource_size)) = session
                .resources
                .iter()
                .find(|it| it.order_id == start_delim.resource_id())
                .map(|it| (it.path.clone(), it.size))
            else {
                return Err(WebRtcErrors::InvalidDelimiter(format!(
                    "Start delimiter not matching any resource: {start_delim:?}"
                )));
            };

            log::info!("Begin downloading resource {:?} size={}", resource_path, resource_size);

            let mut writer = self.resource_repo.write(resource_path.clone(), start_delim.compressed()).await?;

            let Some(progress_update) = session.resource_mut_progress(start_delim.resource_id()) else {
                return Err(anyhow!("Missing progress for resource {}", start_delim.resource_id()).into());
            };

            let mut total_written_bytes = 0u64;

            // Create writer channel and spawn writer task
            let (mut write_tx, mut write_rx) = mpsc::channel::<Packet>(20);
            let writer_cancel_signal = cancellation_signal.clone();
            let writer_core_request = core_request.clone();
            let mut writer_progress_update = progress_update.clone();
            let writer_handle = spawn(async move {
                let mut total_written = 0u64;
                let mut total_time_us = 0u64;
                let mut write_count = 0u64;
                loop {
                    let write_result = write_rx.next()
                        .with_cancel(&writer_cancel_signal)
                        .await;

                    match write_result {
                        Ok(Some(data)) => {
                            let data_len = data.len() as u64;
                            let time = Instant::now();
                            match writer.write(data.into()).await {
                                Ok(written) => {
                                    total_time_us += time.elapsed().as_micros() as u64;
                                    total_written += written as u64;
                                    write_count += 1;

                                    // Update progress and send response
                                    writer_progress_update.update_progress(written as u64);
                                    writer_core_request.response_throttle(TransferResourceProgressUpdate(writer_progress_update.clone())).await;
                                }
                                Err(e) => {
                                    log::error!("Writer error: {:?}", e);
                                    return Err(WebRtcErrors::PersistentError(PersistenceError::IOError(format!("{e:?}"))));
                                }
                            }
                        }
                        _ => {
                            writer_progress_update.success();
                            break
                        }
                    }
                }

                writer.end().await?;
                Result::<(u64, u64, u64, _), WebRtcErrors>::Ok((total_written, total_time_us, write_count, writer_progress_update))
            });

            loop {
                let frame_opt = {
                    let packet_fut = resource_rx.next().with_cancel(&cancellation_signal).fuse();

                    futures::pin_mut!(packet_fut);

                    if let Some(check_time) = next_check_time {
                        let sleep_fut = sleep(check_time.saturating_duration_since(Instant::now())).fuse();
                        futures::pin_mut!(sleep_fut);

                        select_biased! {
                            raw_packet = packet_fut => {
                                let raw_packet = raw_packet
                                    .ok()
                                    .flatten()
                                    .ok_or_else(|| WebRtcErrors::InvalidDelimiter("Stream ended before end delimiter".into()))?;

                                    let frame = Frame::deserialize(&raw_packet);
                                    frame
                            },
                            _ = sleep_fut => {
                                // Timeout expired, ping the FEC receiver
                                None
                            },
                        }
                    } else {
                        // No timeout set, just wait for packets
                        let raw_packet = packet_fut
                            .await
                            .ok()
                            .flatten()
                            .ok_or_else(|| WebRtcErrors::InvalidDelimiter("Stream ended before end delimiter".into()))?;

                        let frame = Frame::deserialize(&raw_packet);
                        frame
                    }
                };

                let action = if let Some(first_frame) = frame_opt {
                    if let Some(rtt) = self.buffer.rtt().await {
                        fec_receiver.set_rtt(rtt as u64);
                    }

                    let mut frames = vec![first_frame];
                    frames.extend_from_slice(resource_rx.drain().collect::<Vec<_>>().as_slice());
                    let time = Instant::now();
                    let action = fec_receiver.receive(frames)?;
                    total_decode_time_us += time.elapsed().as_micros() as u64;
                    total_decode_count += 1;
                    action
                } else {
                    fec_receiver.ping()?
                };

                // Handle the action
                match &action {
                    FecAction::Queued(instant) => {
                        next_check_time = Some(*instant);
                        continue;
                    },
                    FecAction::Terminated => {
                        log::warn!("FecReceiver terminated, will cancel transfer");
                        break;
                    }
                    _ => {}
                }

                let (new_packets, maybe_next_check) = self.handle_fec_action(action).await?;
                if let Some(instant) = maybe_next_check {
                    next_check_time = Some(instant);
                } else {
                    next_check_time = None;
                }

                let mut end_of_stream = false;
                for packet in new_packets {
                    let is_hold = TransferDelimiterShema::from_hold_packet(&packet, session_id).is_ok();
                    let is_end = TransferDelimiterShema::from_end_packet(&packet, session_id).is_ok();
                    let rtt = self.buffer.rtt().await.unwrap_or(0.0);
                    let current_block_id = fec_receiver.current_block_id();
                    network_stats.current_block_id = Some(current_block_id);
                    network_stats.rtt = Some(rtt as u32);
                    if is_hold {
                        let loss_rate = fec_receiver.calculate_loss_rate();
                        network_stats.loss_rate = loss_rate;
                        let feedback = FecFeedback {
                            feedback: Some(Feedback::Network(network_stats.clone())),
                        };

                        self.msg_channel.notify(Request::FecFeedback(feedback)).await?;
                        next_check_time.replace( fec_receiver.hiccup());
                        continue;
                    }
                    else if is_end {
                        end_of_stream = true;
                        next_check_time.replace( fec_receiver.hiccup());
                        log::info!("End delimiter received, total received bytes is {total_byte_received}");
                        break;
                    }

                    // Send to writer channel (progress update now happens in writer task)
                    let data_len = packet.len() as u64;
                    total_byte_received += data_len;
                    total_written_bytes += data_len;
                    total_written_bytes_all += data_len;
                    if write_tx.send(packet).await.is_err() {
                        log::error!("Writer channel closed unexpectedly");
                        return Err(WebRtcErrors::PersistentError(PersistenceError::IOError("Writer channel closed".into())));
                    }
                }

                if end_of_stream {
                    break
                }
            }

            let _ = write_tx.close().await;
            let (written_bytes, write_time_us, write_count, mut writer_progress) = writer_handle.await??;
            total_write_time_us += write_time_us;
            total_write_count += write_count;
            log::info!("Writer task finished, total written: {} bytes in {} chunks", written_bytes, write_count);

            writer_progress.success();
            let _ = core_request.response(TransferResourceProgressUpdate(writer_progress.clone())).await;
            session.update_progress(writer_progress);

            log::info!("Complete downloading resource {:?}, total {} bytes", resource_path, resource_size);

            self.msg_channel.notify(Request::FecFeedback(FecFeedback { feedback: Some(Feedback::Network(network_stats.clone()))})).await?;

            log::info!("Notified stats for end delimiter");
            if total_byte_received > 0 {
                log::info!("Total received bytes is {total_byte_received}")
            }

            total_byte_received = 0;
        }

        // Giving max 10s more for thumbnail to complete
        drop(resource_rx);
        cancellation_signal.cancel_after(Duration::from_secs(10));
        let _ = thumbnail_handle.await;

        // Log average metrics
        if total_decode_count > 0 {
            let avg_decode_time = total_decode_time_us / total_decode_count;
            log::info!("Receiver average decode frame time: {}us (total: {} frames)", avg_decode_time, total_decode_count);
        }
        if total_write_count > 0 {
            let avg_write_time = total_write_time_us / total_write_count;
            log::info!("Receiver average write chunk time: {}us (total: {} chunks, {} bytes)",
                avg_write_time, total_write_count, total_written_bytes_all);
        }

        Ok(session.status())
    }

    async fn handle_fec_action(&self, action: FecAction) -> Result<(Vec<Packet>, Option<Instant>), WebRtcErrors> {
        match action {
            FecAction::Constructed(packets, next_check) => Ok((packets, Some(next_check))),
            FecAction::Feedback(fb, next_check) => {
                log::info!("Sending FEC feedback: {:?}", fb);
                self.msg_channel.notify(Request::FecFeedback(fb)).await?;
                Ok((vec![], Some(next_check)))
            }
            FecAction::Terminated => {
                log::warn!("FEC terminated");
                Err(WebRtcErrors::InvalidDelimiter("FEC terminated".into()))
            }
            FecAction::Queued(_) | FecAction::Noop => Ok((vec![], None)), // Ignore queued and noop
            _ => Ok((vec![], None)), // Ignore others
        }
    }

    pub async fn transfer_session(
        &self,
        core_request: CoreRequest,
        mut session: TransferSession,
    ) -> Result<TransferSessionStatus, WebRtcErrors> {
        let request_id = uuid::Uuid::now_v7();
        let cancellation_signal = session.token().clone();
        self.transfers_context.start_transfer(session.order_id, request_id.to_string()).await;
        self.transfers_context.add_token(session.order_id, cancellation_signal.clone()).await;

        let _drop_guard = cancellation_signal.drop_guard();

        let session_id = session.order_id;
        log::info!(
            "Requesting peer to transfer session {session_id}, thumbnails{:?}",
            session.resources.iter().map(|r| r.thumbnail_path.clone()).collect::<Vec<_>>()
        );

        for resource in session.resources.iter_mut() {
            if matches!(resource.r#type, ResourceType::Folder) {
                resource.name = format!("{}.zip", resource.name);
            }
        }

        let transfer_session_message = TransferSessionMessage {
            order_id: session.order_id,
            resources: session.resources.iter().map(|r| r.to_proto()).collect(),
        };

        let peer_id = session.peer().map(|it| it.peer_id()).context("This is not a peer session")?;
        let request = Request::TransferRequest(TransferRequestMessage {
            session: transfer_session_message,
        });

        let response = self.msg_channel.send(request, Some(request_id)).await?;
        log::info!("Received response for session {session_id} {response:?}");

        let buffer = self.buffer.clone();
        let repo = self.resource_repo.clone();

        let thumbnail_handle = {
            let mut session_thumbnail_paths = session
                .resources
                .iter()
                .filter_map(|r| r.thumbnail_path.clone().map(|it| (r.order_id, it)))
                .rev()
                .collect::<Vec<_>>();
            let thumbnail_channel = self.thumbnail_channel.clone();

            let thumbnail_cancel_signal = cancellation_signal.clone();
            log::info!(
                "Begin sending {} thumbnails for session {session_id}",
                session_thumbnail_paths.len()
            );
            spawn(async move {
                while let Some((id, thumbnail_path)) = session_thumbnail_paths.pop() {
                    let Ok(Ok(mut reader)) =
                        repo.read(thumbnail_path.clone(), 63 * 1024, false).with_cancel(&thumbnail_cancel_signal).await
                    else {
                        log::warn!("Found thumbnail path {thumbnail_path:?} for resource {id} but it does not exist, skipping");
                        continue;
                    };

                    log::info!("Begin sending thumbnail for resource {id} {thumbnail_path:?}");

                    let begin_delimiter = TransferDelimiterShema::start(session_id, id, false).as_bytes()?;

                    if let Err(e) = thumbnail_channel.unbounded_send((peer_id, begin_delimiter)) {
                        log::error!("Failed to send delimiter to peer for thumbnail {peer_id:?}: {e:?}");
                        return Err(WebRtcErrors::PersistentError(PersistenceError::IOError(format!("{e:?}"))));
                    }

                    while let Ok(Ok(Some(bytes))) = reader.next(None).with_cancel(&thumbnail_cancel_signal).await {
                        let bytes = Packet::from(bytes);
                        if !bytes.is_empty() {
                            let _ = thumbnail_channel.unbounded_send((peer_id, bytes));
                        }

                        buffer.wait_buffer_low(TRANSFER_THUMBNAIL_CHANNEL_ID, MIN_BUFFER_SIZE / 2, Duration::from_millis(300)).await;
                    }

                    let end_delimiter = TransferDelimiterShema::end(session_id, id, false).as_bytes()?;
                    if let Err(e) = thumbnail_channel.unbounded_send((peer_id, end_delimiter)) {
                        log::error!("Failed to send delimiter to peer for thumbnail {peer_id:?}: {e:?}");
                        return Err(WebRtcErrors::PersistentError(PersistenceError::IOError(format!("{e:?}"))));
                    }
                }

                Ok(session_thumbnail_paths)
            })
        };

        let resource_cancel_signal = cancellation_signal.clone();
        let mut feedback_receiver = self.transfer_feedback_receiver.retrieve().await?;
        let _ = feedback_receiver.drain();

        // Metrics tracking for averages
        let mut total_read_bytes = 0u64;
        let mut total_read_time_us = 0u64;
        let mut total_frame_build_time_us = 0u64;
        let mut total_frame_build_count = 0u64;

        while !session.is_completed() {
            let Some((resource_path, order_id, size, name)) = session
                .get_next_transfer_resource()
                .map(|it| (it.path.clone(), it.order_id, it.size, it.name.clone()))
            else {
                break;
            };

            let is_compressed = is_compressible(name.as_str());
            let chunk_size = if is_compressed {
                (CHUNK_SIZE * DATA_SHARDS_DEFAULT - 1150) as u64
            } else {
                (CHUNK_SIZE * DATA_SHARDS_DEFAULT) as u64
            };

            let mut reader = Arc::new(Mutex::new(self
                .resource_repo
                .read(resource_path.clone(), chunk_size as usize, is_compressed)
                .with_cancel(&resource_cancel_signal)
                .await??));

            log::info!("Begin transferring resource {resource_path:?} size {size} bytes compressed = {is_compressed}");

            let mut total_data_sent = 0u64;
            let mut total_sent_bytes = 0u64;
            let Some(progress_update) = session.resource_mut_progress(order_id) else {
                return Err(anyhow!("Missing progress for resource {}", order_id).into());
            };

            let (mut read_tx, mut read_rx) = mpsc::channel::<(Packet, usize)>(20);

            let reader_cancel_signal = resource_cancel_signal.clone();
            let reader2 = reader.clone();
            let reader_handle = spawn(async move {
                let mut total_read = 0u64;
                let mut total_time_us = 0u64;
                loop {
                    let time = Instant::now();
                    let mut reader = reader2.lock().await;
                    let read_result = reader.c_next(Some(chunk_size))
                        .with_cancel(&reader_cancel_signal)
                        .await;

                    match read_result {
                        Ok(Ok(Some((data, raw_size)))) => {
                            total_time_us += time.elapsed().as_micros() as u64;
                            total_read += data.len() as u64;
                            let packet = Packet::from(data);
                            drop(reader);

                            // Send to channel, blocking if full
                            if read_tx.send((packet, raw_size)).await.is_err() {
                                log::info!("Reader channel closed, stopping reader task");
                                break;
                            }
                        }
                        Ok(Ok(None)) => {
                            // End of file
                            log::info!("Reader task completed, total read: {} bytes", total_read);
                            break;
                        }
                        Ok(Err(e)) => {
                            log::error!("Reader error: {:?}", e);
                            break;
                        }
                        Err(_) => {
                            // Cancelled
                            log::info!("Reader task cancelled");
                            break;
                        }
                    }
                }

                Result::<(u64, u64), WebRtcErrors>::Ok((total_read, total_time_us))
            });

            let mut on_hold = true;
            let mut is_end = false;

            // Send start delimiter directly without FEC encoding
            let mut fec_sender = FecSender::new(self.peer.peer_id(), 512);
            let delimiter = TransferDelimiterShema::start(session_id, order_id, is_compressed).as_bytes()?;
            let _ = self.reliable_data_channel.unbounded_send((self.peer.peer_id(), delimiter));

            let mut buff_counter = 0;
            let _ = self.buffer.flush_timeout(TRANSFER_RESOURCE_RELIABLE_CHANNEL_ID).await;

            let mut received_from_readers = 0;
            loop {
                let (bytes, raw_size, feedback) = {
                    if on_hold || is_end {
                        let timeout_fut = sleep(Duration::from_secs(10)).fuse();
                        let fb_fut = feedback_receiver.next().with_cancel(&resource_cancel_signal).fuse();

                        futures::pin_mut!(timeout_fut);
                        futures::pin_mut!(fb_fut);
                        select_biased! {
                            _ = timeout_fut => {
                                return Err(anyhow!("Timeout waiting for feedback while on hold, resuming transfer").into());
                            },
                            fb = fb_fut => {
                                let fb = fb?;
                                fb.map(|f| (None, None, Some(f))).unwrap_or((None, None, None))
                            },
                        }
                    }
                    else {
                        let reader_fut = read_rx.next().fuse();  // Read from channel instead of direct reader
                        let fb_fut = feedback_receiver.next().with_cancel(&resource_cancel_signal).fuse();

                        futures::pin_mut!(fb_fut);
                        futures::pin_mut!(reader_fut);
                        select_biased! {
                            r = reader_fut => {
                                // Received from reader task channel
                                r.map(|it| (Some(it.0), Some(it.1), None)).unwrap_or((None, None, None))
                            },
                            fb = fb_fut => {
                                let fb = fb?;
                                fb.map(|f| (None, None, Some(f))).unwrap_or((None, None, None))
                            },
                        }
                    }
                };

                if let Some(rtt) = self.buffer.rtt().await {
                    fec_sender.set_rtt(rtt as u64);
                }

                let action = match (bytes, raw_size, feedback) {
                    (Some(bytes), Some(raw_size), _) => {
                        let time = Instant::now();
                        received_from_readers += bytes.len();
                        progress_update.update_progress(raw_size as u64);
                        let action = fec_sender.send(bytes)?;
                        total_frame_build_time_us += time.elapsed().as_micros() as u64;
                        total_frame_build_count += 1;
                        let _ = core_request.response_throttle(TransferResourceProgressUpdate(progress_update.clone())).await;
                        action
                    }
                    (_, _, Some(fb)) => {
                        if let Feedback::Network(ref net) = fb {
                            if on_hold {
                                log::info!("Received network feedback while on hold: loss_rate={}, rtt={:?}, block_id={:?}",
                                    net.loss_rate, net.rtt, net.current_block_id);

                                if is_end {
                                    log::info!("End delimiter acknowledged, finishing resource transfer");
                                    break;
                                }

                                on_hold = false;
                            }
                        }
                        fec_sender.feedback(fb)
                    }
                    _ => {
                        self.quad_unreliable_channel.lock().await.flush_timeout().await?;
                        self.buffer.flush_timeout(TRANSFER_RESOURCE_RELIABLE_CHANNEL_ID).await?;

                        // Send end delimiter directly without FEC encoding
                        let end_delimiter = TransferDelimiterShema::end(session_id, order_id, is_compressed).as_bytes()?;
                        log::info!("No data left for resource {resource_path:?}, sending end delimiter");

                        on_hold = true;
                        is_end = true;
                        fec_sender.send(end_delimiter)?
                    }
                };

                match action {
                    FecAction::Framed(frames) => {
                        let mut quad_channel = self.quad_unreliable_channel.lock().await;
                        for frame in frames {
                            let packet = frame.serialize();
                            total_data_sent += frame.data().len() as u64;
                            total_sent_bytes += packet.len() as u64;
                            buff_counter += packet.len();
                            let _ = quad_channel.send(self.peer.peer_id(), packet);
                        }
                    }
                    FecAction::Retransmit(frames) => {
                        if !frames.is_empty() {
                            log::info!("Retransmitting packet: {:?} block", frames[0].block_id);
                            for frame in frames {
                                let packet = frame.serialize();
                                buff_counter += packet.len();
                                let _ = self.reliable_data_channel.unbounded_send((self.peer.peer_id(), packet));
                            };
                        }
                    }
                    FecAction::Terminated => {
                        log::info!("Fec sender terminated, aborting resource transfer");
                        break;
                    }
                    FecAction::Noop | FecAction::Feedback(_, _) | FecAction::Constructed(_, _) | FecAction::Queued(_) => {
                        // Will not happens, do nothing
                    }
                };

                if buff_counter > 4 * MAX_BUFFER_SIZE {
                    let mut should_send_hold = false;
                    if !on_hold {
                        on_hold = true;
                        should_send_hold = true;
                    }

                    let tick = Instant::now();
                    let quad_ch = self.quad_unreliable_channel.lock().await;
                    let stats_before = quad_ch.bytes_sent().await;

                    quad_ch.wait_buffer_low(MIN_BUFFER_SIZE, Duration::from_millis(1500)).await;

                    self.buffer.wait_buffer_low(TRANSFER_RESOURCE_RELIABLE_CHANNEL_ID, MIN_BUFFER_SIZE, Duration::from_millis(4 * fec_sender.rtt().max(MIN_BUFFER_SIZE as u64))).await;

                    if should_send_hold {
                        let hold_delimiter = TransferDelimiterShema::hold(session_id, order_id).as_bytes()?;
                        let FecAction::Framed(frames) = fec_sender.send(hold_delimiter)? else {
                            return Err(anyhow!("Failed to build hold delimiter").into());
                        };

                        for frame in frames {
                            let _ = self.reliable_data_channel.unbounded_send((self.peer.peer_id(), frame.serialize()));
                        }
                    };

                    let time = tick.elapsed().as_secs_f64().max(f64::MIN);
                    let stats_after = quad_ch.bytes_sent().await;
                    let total_sent = stats_after.saturating_sub(stats_before);
                    let bw = total_sent as f64 / time;
                    if (bw > 1f64) {
                        reader.lock().await.compression_stats_mut().update_network_bandwidth(bw);
                        log::info!("Buffer low, sent {} bytes in {} seconds, bandwidth: {:.2} kbps", total_sent, time, bw / 1000.0);
                    }

                    buff_counter = 0;
                }
            }

            progress_update.success();
            let _ = core_request.response_throttle(TransferResourceProgressUpdate(progress_update.clone())).await;

            read_rx.close();
            drop(read_rx);
            match reader_handle.await {
                Ok(Ok((read_bytes, read_time_us))) => {
                    total_read_bytes += read_bytes;
                    total_read_time_us += read_time_us;
                    log::info!("Reader task finished, total read: {} bytes", read_bytes);
                }
                Ok(Err(e)) => {
                    log::warn!("Reader task finished with error: {:?}", e);
                }
                Err(e) => {
                    log::warn!("Reader task join error: {:?}", e);
                }
            }

            log::info!(
                "Complete transferring resource {resource_path:?} with status {:?} total_sent {:?} total_data {:?}, reader {}",
                progress_update.status,
                total_sent_bytes,
                total_data_sent,
                received_from_readers
            );

            received_from_readers = 0;
            total_sent_bytes = 0;
            total_data_sent = 0;
        }

        // Giving max 10s more for thumbnail to complete
        cancellation_signal.cancel_after(Duration::from_secs(10));
        let _ = thumbnail_handle.await;
        self.buffer.flush_all_timeout().await?;

        // Log average metrics
        if total_read_time_us > 0 && total_read_bytes > 0 {
            let avg_read_speed = (total_read_bytes as f64 * 1_000_000.0) / total_read_time_us as f64;
            log::info!("Sender average reading data speed: {:.2} Byte/s (total: {} bytes in {}us)",
                avg_read_speed, total_read_bytes, total_read_time_us);
        }
        if total_frame_build_count > 0 {
            let avg_build_time = total_frame_build_time_us / total_frame_build_count;
            log::info!("Sender average build frame time: {}us (total: {} frames)",
                avg_build_time, total_frame_build_count);
        }

        log::info!("Transfer session {session_id} completed");

        Ok(session.status())
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
}

impl Drop for WebRtcPeer {
    fn drop(&mut self) {
        log::info!("Dropped peer {:?}", self.peer.peer_id());
    }
}
