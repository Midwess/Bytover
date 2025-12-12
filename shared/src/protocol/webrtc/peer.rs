use crate::app::operations::p2p::P2POperationOutput;
use crate::app::operations::transfer::TransferOperationOutput;
use crate::app::operations::transfer::TransferOperationOutput::TransferResourceProgressUpdate;
use crate::app::operations::CoreOperationOutput;
use crate::entities::local_resource::{LocalResourcePath, ResourceType};
use crate::entities::peer::Peer as PeerEntity;
use crate::entities::transfer_session::{ThumbnailUpdatedEvent, TransferSession, TransferSessionStatus};
use crate::protocol::webrtc::errors::WebRtcErrors;
use crate::protocol::webrtc::fec::{FecAction, FecReceiver, FecSender, Frame};
use crate::protocol::webrtc::message_channel::DirectMessageChannel;
use crate::protocol::webrtc::transfer::{TransferDelimiterShema, TransfersContext};
use crate::protocol::webrtc::webrtc::{MAX_BUFFER_SIZE, MIN_BUFFER_SIZE, TRANSFER_RESOURCE_RELIABLE_CHANNEL_ID, TRANSFER_RESOURCE_UNRELIABLE_CHANNEL_ID, TRANSFER_THUMBNAIL_CHANNEL_ID};
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
    BeginTransferResource, CancelTransferSessionRequest, EndTransferResource, IntroduceRequestMessage, IntroduceResponseMessage,
    PeerMessage, TransferRequestMessage, TransferResponseMessage, TransferSessionMessage,
};
use std::sync::mpsc::channel;
use std::sync::Arc;
use std::time::Duration;

pub struct WebRtcPeer {
    pub peer: PeerEntity,
    pub resource_repo: Arc<dyn LocalResourceRepository>,

    // Channel used to communicate with the peer
    pub msg_channel: DirectMessageChannel,
    // Channel used to transfer the resource
    pub unreliable_data_channel: mpsc::UnboundedSender<(PeerId, Packet)>,
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

        Ok(Self {
            msg_channel,
            peer,
            transfer_feedback_receiver: YieldContainer::new(transfer_feedback_receiver),
            transfer_feedback_sender,
            reliable_data_channel,
            unreliable_data_channel,
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


        Ok(Self {
            msg_channel,
            transfer_feedback_sender,
            transfer_feedback_receiver: YieldContainer::new(transfer_feedback_receiver),
            peer: msg.mine.into(),
            reliable_data_channel,
            unreliable_data_channel,
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

        let mut fec_receiver = FecReceiver::new();
        let mut start_delimiter_tick = Instant::now();
        let mut next_check_time: Option<Instant> = None;

        loop {
            if session.is_completed() {
                log::warn!("Session {session_id} is completed");
                break;
            }

            // Wait for either a new packet or a timeout
            let frame = {
                let packet_fut = resource_rx.next().with_cancel(&cancellation_signal).fuse();

                futures::pin_mut!(packet_fut);

                if let Some(check_time) = next_check_time {
                    let sleep_fut = sleep(check_time.saturating_duration_since(Instant::now())).fuse();
                    futures::pin_mut!(sleep_fut);

                    select_biased! {
                        raw_packet = packet_fut => {
                            let Some(raw_packet) = raw_packet? else {
                                break;
                            };

                            let Some(frame) = Frame::deserialize(&raw_packet) else {
                                log::warn!("Failed to deserialize packet: {raw_packet:?}");
                                continue;
                            };

                            Some(frame)
                        },
                        _ = sleep_fut => {
                            // Timeout expired, ping the FEC receiver
                            None
                        },
                    }
                } else {
                    // No timeout set, just wait for packets
                    let Some(raw_packet) = packet_fut.await? else {
                        break;
                    };

                    let Some(frame) = Frame::deserialize(&raw_packet) else {
                        log::warn!("Failed to deserialize packet: {raw_packet:?}");
                        continue;
                    };

                    Some(frame)
                }
            };

            let action = if let Some(frame) = frame {
                // Process received frame
                if let Some(rtt) = self.buffer.rtt().await {
                    fec_receiver.set_rtt(rtt as u64);
                }
                fec_receiver.receive(frame)?
            } else {
                // Timeout expired, ping for feedback
                fec_receiver.ping()?
            };

            // Handle the action
            match action {
                FecAction::Queued(instant) => {
                    next_check_time = Some(instant);
                    continue;
                },
                _ => {
                    next_check_time = None;
                }
            }

            let mut packets = self.handle_fec_action(action).await?;
            if packets.is_empty() {
                continue;
            }

            // Check for hold delimiter first
            let start_delim = loop {
                let Some(packet) = packets.pop() else {
                    break Err(WebRtcErrors::InvalidDelimiter("No delimiter found in FEC packets".into()));
                };

                if let Ok(delimiter) = TransferDelimiterShema::from_start_packet(&packet, session_id) {
                    break Ok(delimiter)
                } else {
                    continue;
                }
            };

            let Ok(start_delim) = start_delim else {
                if start_delimiter_tick.elapsed() > Duration::from_secs(10) {
                    return Err(WebRtcErrors::InvalidDelimiter("Failed to parse start delimiter from packet".into()));
                }

                continue;
            };

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

            let mut packets = packets;
            loop {
                if progress_update.is_completed() {
                    break;
                }

                // Wait for either a new packet or a timeout
                let frame = {
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

                                let Some(frame) = Frame::deserialize(&raw_packet) else {
                                    return Err(WebRtcErrors::InvalidDelimiter("Failed to deserialize packet".into()));
                                };

                                Some(frame)
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

                        let Some(frame) = Frame::deserialize(&raw_packet) else {
                            return Err(WebRtcErrors::InvalidDelimiter("Failed to deserialize packet".into()));
                        };

                        Some(frame)
                    }
                };

                let action = if let Some(frame) = frame {
                    // Process received frame
                    if let Some(rtt) = self.buffer.rtt().await {
                        fec_receiver.set_rtt(rtt as u64);
                    }

                    let time = Instant::now();
                    let action = fec_receiver.receive(frame)?;
                    log::info!("Received FEC {}us", time.elapsed().as_micros());
                    action
                } else {
                    // Timeout expired, ping for feedback
                    fec_receiver.ping()?
                };

                // Handle the action
                match &action {
                    FecAction::Queued(instant) => {
                        next_check_time = Some(*instant);
                        continue;
                    },
                    _ => {
                        next_check_time = None;
                    }
                }

                packets.extend_from_slice(self.handle_fec_action(action).await?);

                for packet in packets.drain(..) {
                    if let Ok(_hold) = TransferDelimiterShema::from_hold_packet(&packet, session_id) {
                        log::info!("Received hold delimiter, sending network feedback");

                        // Calculate statistics
                        let loss_rate = fec_receiver.calculate_loss_rate();
                        let rtt = self.buffer.rtt().await.unwrap_or(0.0);
                        let current_block_id = fec_receiver.current_block_id();

                        // Send Feedback::Network back to sender
                        use schema::devlog::bitbridge::{FecFeedback, NetworkStats};
                        use schema::devlog::bitbridge::fec_feedback::Feedback;

                        let feedback = FecFeedback {
                            feedback: Some(Feedback::Network(NetworkStats {
                                loss_rate,
                                rtt: Some(rtt as u32),
                                current_block_id: Some(current_block_id),
                            })),
                        };

                        self.msg_channel.notify(Request::FecFeedback(feedback)).await?;
                        log::info!("Sent network feedback: loss_rate={}, rtt={}, block_id={}",
                            loss_rate, rtt, current_block_id);

                        continue;
                    }

                    if TransferDelimiterShema::from_end_packet(&packet, session_id).is_ok() {
                        log::info!("End delimiter received, finishing download");
                        progress_update.success();
                        break;
                    }

                    // Write chunk
                    let time = Instant::now();
                    let written = packet.len();
                    let written = writer
                        .write(packet.clone().into())
                        .with_cancel(&cancellation_signal)
                        .await?
                        .map_err(|it| WebRtcErrors::PersistentError(PersistenceError::IOError(format!("{it:?}"))))?;
                    log::info!("Wrote {} bytes in {}us", written, time.elapsed().as_micros());

                    total_written_bytes += written as u64;
                    progress_update.update_progress(written as u64);

                    core_request.response_throttle(TransferResourceProgressUpdate(progress_update.clone())).await;
                }
            }

            writer.end().await?;
            progress_update.complete();
            let _ = core_request.response(TransferResourceProgressUpdate(progress_update.clone())).await;
            start_delimiter_tick = Instant::now();

            log::info!(
                "Complete downloading resource {:?}, total {} bytes",
                resource_path,
                total_written_bytes
            );
        }

        // Giving max 10s more for thumbnail to complete
        drop(resource_rx);
        cancellation_signal.cancel_after(Duration::from_secs(10));
        let _ = thumbnail_handle.await;

        Ok(session.status())
    }

    async fn handle_fec_action(&self, action: FecAction) -> Result<Vec<Packet>, WebRtcErrors> {
        match action {
            FecAction::Constructed(packets) => Ok(packets),
            FecAction::Feedback(fb) => {
                log::info!("Sending FEC feedback: {:?}", fb);
                self.msg_channel.notify(Request::FecFeedback(fb)).await?;
                Ok(vec![])
            }
            FecAction::Terminated => {
                log::warn!("FEC terminated");
                Err(WebRtcErrors::InvalidDelimiter("FEC terminated".into()))
            }
            FecAction::Queued(_) | FecAction::Noop => Ok(vec![]), // Ignore queued and noop
            _ => Ok(vec![]), // Ignore others
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
        let mut fec_sender = FecSender::new(self.peer.peer_id(), 5 * 1024 * 1024);
        let mut feedback_receiver = self.transfer_feedback_receiver.retrieve().await?;
        let _ = feedback_receiver.drain();
        while !session.is_completed() {
            let Some((resource_path, order_id, size, name)) = session
                .get_next_transfer_resource()
                .map(|it| (it.path.clone(), it.order_id, it.size, it.name.clone()))
            else {
                break;
            };

            let is_compressed = false;
            let chunk_size = 100 * 1024;

            let mut reader = self
                .resource_repo
                .read(resource_path.clone(), chunk_size as usize, is_compressed)
                .with_cancel(&resource_cancel_signal)
                .await??;

            log::info!("Begin transferring resource {resource_path:?} size {size} bytes");

            let mut total_sent_bytes = 0u64;
            let Some(progress_update) = session.resource_mut_progress(order_id) else {
                return Err(anyhow!("Missing progress for resource {}", order_id).into());
            };

            let (queue_tx, mut queue_rx) = mpsc::unbounded();
            let mut on_hold = false;

            let delimiter = TransferDelimiterShema::start(session_id, order_id, is_compressed).as_bytes()?;
            let FecAction::Framed(fec) = fec_sender.send(delimiter)? else {
                return Err(anyhow!("Failed to send delimiter to FEC sender").into());
            };

            let packets = fec.into_iter().map(|it| it.serialize()).collect::<Vec<_>>();
            for packet in packets {
                let _ = self.reliable_data_channel.unbounded_send((self.peer.peer_id(), packet));
            }
            let mut buff_counter = 0;
            let time_to_drain = Duration::from_secs(5);

            let _ = self.buffer.flush_timeout(TRANSFER_RESOURCE_RELIABLE_CHANNEL_ID).await;
            loop {
                let (bytes, raw_size, feedback) = {
                    if on_hold {
                        // When on hold, only wait for feedback with a timeout to prevent hanging
                        let timeout_fut = sleep(Duration::from_secs(30)).fuse();
                        let fb_fut = feedback_receiver.next().with_cancel(&resource_cancel_signal).fuse();

                        futures::pin_mut!(timeout_fut);
                        futures::pin_mut!(fb_fut);
                        select_biased! {
                            _ = timeout_fut => {
                                log::warn!("Timeout waiting for feedback while on hold, resuming transfer");
                                on_hold = false;
                                (None, None, None)
                            },
                            fb = fb_fut => {
                                let fb = fb?;
                                fb.map(|f| (None, None, Some(f))).unwrap_or((None, None, None))
                            },
                        }
                    }
                    else {
                        let queue_fut = queue_rx.next().fuse();
                        let reader_fut = reader.c_next(Some(chunk_size)).with_cancel(&resource_cancel_signal).fuse();
                        let fb_fut = feedback_receiver.next().with_cancel(&resource_cancel_signal).fuse();

                        futures::pin_mut!(queue_fut);
                        futures::pin_mut!(fb_fut);
                        futures::pin_mut!(reader_fut);
                        select_biased! {
                            q = queue_fut => {
                                let q: Option<(Packet, usize)> = q;
                                q.map(|it| (Some(it.0), Some(it.1), None)).unwrap_or((None::<Packet>, None::<usize>, None::<Feedback>))
                            },
                            r = reader_fut => {
                                let res = r??;
                                res.map(|it| (Some(Packet::from(it.0)), Some(it.1), None)).unwrap_or((None, None, None))
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
                        total_sent_bytes += bytes.len() as u64;
                        let time = Instant::now();
                        let action = fec_sender.send(bytes)?;
                        progress_update.update_progress(raw_size as u64);
                        let _ = core_request.response_throttle(TransferResourceProgressUpdate(progress_update.clone())).await;
                        action
                    }
                    (_, _, Some(fb)) => {
                        if let Feedback::Network(ref net) = fb {
                            if on_hold {
                                on_hold = false;
                                log::info!("Received network feedback, marking on_hold=false: loss_rate={}, rtt={:?}, block_id={:?}",
                                    net.loss_rate, net.rtt, net.current_block_id);
                            }
                        }
                        fec_sender.feedback(fb)
                    }
                    _ => {
                        log::info!("End of resource {resource_path:?}");
                        break;
                    }
                };

                match action {
                    FecAction::Framed(frames) => {
                        for frame in frames {
                            let packet = frame.serialize();
                            buff_counter += packet.len();
                            let _ = self.unreliable_data_channel.unbounded_send((self.peer.peer_id(), packet));
                        }
                    }
                    FecAction::Retransmit(frames) => {
                        for frame in frames {
                            log::info!("Retransmitting packet: {:?} frame {:?}", frame.block_id, frame.frame_idx);
                            let packet = frame.serialize();
                            buff_counter += packet.len();
                            let _ = self.reliable_data_channel.unbounded_send((self.peer.peer_id(), packet));
                        };
                    }
                    FecAction::Terminated => {
                        log::info!("Fec sender terminated, aborting resource transfer");
                        break;
                    }
                    FecAction::Noop | FecAction::Feedback(_) | FecAction::Constructed(_) | FecAction::Queued(_) => {
                        // Will not happens, do nothing
                    }
                };

                if buff_counter > MAX_BUFFER_SIZE  {
                    log::info!("Buffer full at block = {}, marking on_hold=true and sending hold delimiter for session {}", fec_sender.block_id, session_id);
                    if !on_hold {
                        on_hold = true;
                        let hold_delimiter = TransferDelimiterShema::hold(session_id, order_id).as_bytes()?;
                        let FecAction::Framed(fec_frames) = fec_sender.send(hold_delimiter)? else {
                            return Err(anyhow!("Failed to send hold delimiter").into());
                        };

                        for frame in fec_frames {
                            let _ = self.reliable_data_channel.unbounded_send((self.peer.peer_id(), frame.serialize()));
                        }
                    }

                    reader.compression_stats_mut().start_over();

                    let (mut tx, mut rx) = mpsc::channel(1);
                    let flushed_fut = async {
                        let tick = Instant::now();
                        let stats = self
                            .buffer
                            .channel_bytes_sent_received(TRANSFER_RESOURCE_UNRELIABLE_CHANNEL_ID)
                            .await
                            .map(|it| it.0)
                            .unwrap_or(0);
                        self.buffer.wait_buffer_low(TRANSFER_RESOURCE_UNRELIABLE_CHANNEL_ID, MIN_BUFFER_SIZE, Duration::from_millis(100)).await;
                        let time = tick.elapsed().as_secs_f64().max(f64::MIN);
                        let new_stats = self
                            .buffer
                            .channel_bytes_sent_received(TRANSFER_RESOURCE_UNRELIABLE_CHANNEL_ID)
                            .await
                            .map(|it| it.0)
                            .unwrap_or(0);
                        let bw = (0 - 0) as f64 / time;
                        let _ = tx.send(()).await;
                        Result::<f64, WebRtcErrors>::Ok(bw)
                    };

                    let send_fut = async {
                        let mut total_queued = 0;
                        loop {
                            if total_queued >= MAX_BUFFER_SIZE {
                                break Ok(());
                            };

                            match rx.try_next() {
                                Ok(Some(_)) => break Result::<(), WebRtcErrors>::Ok(()),
                                _ => {}
                            };

                            let Some((data, raw_size)) =
                                reader.c_next(Some(chunk_size)).with_cancel(&cancellation_signal).await??
                            else {
                                break Ok(());
                            };

                            total_queued += data.len();
                            let _ = queue_tx.unbounded_send((Packet::from(data), raw_size));
                        }
                    };

                    let (flushed, send) = join!(flushed_fut, send_fut);
                    let bw = flushed?;
                    log::info!("Bandwidth for resource {resource_path:?} is {bw} bytes/s");
                    buff_counter = 0;
                    send?;
                    reader.compression_stats_mut().update_network_bandwidth(bw);
                }
            }

            self.buffer.flush_timeout(TRANSFER_RESOURCE_UNRELIABLE_CHANNEL_ID).await?;
            self.buffer.flush_timeout(TRANSFER_RESOURCE_RELIABLE_CHANNEL_ID).await?;
            let end_delimiter = TransferDelimiterShema::end(session_id, order_id, is_compressed).as_bytes()?;
            let FecAction::Framed(frames) = fec_sender.send(end_delimiter)? else {
                return Err(WebRtcErrors::InvalidDelimiter("Failed to send end delimiter".into()));
            };

            log::info!("Sending end delimiter for resource {resource_path:?} size {size} bytes");
            for frame in frames {
                let _ = self.reliable_data_channel.unbounded_send((self.peer.peer_id(), frame.serialize()));
            }

            progress_update.complete();
            let _ = core_request.response_throttle(TransferResourceProgressUpdate(progress_update.clone())).await;

            log::info!(
                "Complete transferring resource {resource_path:?} with status {:?} total_sent {:?}",
                progress_update.status,
                total_sent_bytes,
            );
        }

        // Giving max 10s more for thumbnail to complete
        cancellation_signal.cancel_after(Duration::from_secs(10));
        let _ = thumbnail_handle.await;
        self.buffer.flush_all_timeout().await?;
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
