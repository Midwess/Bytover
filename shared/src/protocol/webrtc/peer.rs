use crate::app::operations::p2p::P2POperationOutput;
use crate::app::operations::transfer::TransferOperationOutput;
use crate::app::operations::transfer::TransferOperationOutput::TransferResourceProgressUpdate;
use crate::app::operations::CoreOperationOutput;
use crate::entities::local_resource::{LocalResourcePath, ResourceType};
use crate::entities::peer::Peer as PeerEntity;
use crate::entities::transfer_session::{ThumbnailUpdatedEvent, TransferSession, TransferSessionStatus};
use crate::protocol::webrtc::errors::WebRtcErrors;
use crate::protocol::webrtc::fec::{FecAction, FecSender};
use futures_util::FutureExt;
use crate::protocol::webrtc::message_channel::DirectMessageChannel;
use crate::protocol::webrtc::transfer::{TransferDelimiterShema, TransfersContext};
use crate::protocol::webrtc::webrtc::{MAX_BUFFER_SIZE, TRANSFER_RESOURCE_UNRELIABLE_CHANNEL_ID, TRANSFER_THUMBNAIL_CHANNEL_ID};
use crate::repository::errors::PersistenceError;
use crate::repository::local_resource::LocalResourceRepository;
use crate::shell::api::{BufferExt, CoreRequest};
use crate::utils::compression::is_compressible;
use anyhow::{anyhow, Context};
use core_services::utils::cancellation::FutureExtension;
use core_services::utils::yield_container::YieldContainer;
use futures::channel::mpsc;
use futures::channel::mpsc::unbounded;
use futures::StreamExt;
use futures_util::lock::Mutex;
use futures_util::{join, select, select_biased, SinkExt};
use matchbox_protocol::PeerId;
use matchbox_socket::{Packet, PeerBuffered};
use n0_future::task::spawn;
use n0_future::time::Instant;
use once_cell::sync::OnceCell;
use schema::devlog::bitbridge::fec_feedback::Feedback;
use schema::devlog::bitbridge::peer_message_body::Response::IntroduceResponse;
use schema::devlog::bitbridge::peer_message_body::{Request, Response};
use schema::devlog::bitbridge::{
    CancelTransferSessionRequest, IntroduceRequestMessage, IntroduceResponseMessage, PeerMessage, TransferDelimiter,
    TransferRequestMessage, TransferResponseMessage, TransferSessionMessage,
};
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

    pub fec_sender: Arc<Mutex<FecSender>>,
    pub transfer_feedback_receiver: YieldContainer<mpsc::UnboundedReceiver<Feedback>>,
    pub transfer_feedback_sender: mpsc::UnboundedSender<Feedback>,
    pub transfer_delimiter_receiver: YieldContainer<mpsc::UnboundedReceiver<TransferDelimiter>>,
    pub transfer_delimiter_sender: mpsc::UnboundedSender<TransferDelimiter>,

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
        let (transfer_delimiter_sender, transfer_delimiter_receiver) = unbounded();
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
        let fec_sender = Arc::new(Mutex::new(FecSender::new(peer.peer_id(), 5 * 1024 * 1024)));

        Ok(Self {
            msg_channel,
            peer,
            transfer_feedback_receiver: YieldContainer::new(transfer_feedback_receiver),
            transfer_delimiter_sender,
            transfer_feedback_sender,
            transfer_delimiter_receiver: YieldContainer::new(transfer_delimiter_receiver),
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
            fec_sender,
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
        let (transfer_delimiter_sender, transfer_delimiter_receiver) = unbounded();
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

        let fec_sender = Arc::new(Mutex::new(FecSender::new(user.peer_id(), 5 * 1024 * 1024)));

        Ok(Self {
            fec_sender,
            msg_channel,
            transfer_feedback_sender,
            transfer_delimiter_sender,
            transfer_feedback_receiver: YieldContainer::new(transfer_feedback_receiver),
            transfer_delimiter_receiver: YieldContainer::new(transfer_delimiter_receiver),
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
                    let _ = self.transfer_feedback_sender.unbounded_send(feedback);
                };
            }
            Request::TransferDelimiter(delimiter) => {
                let _ = self.transfer_delimiter_sender.unbounded_send(delimiter);
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

        let cancellation_signal = session.token().clone();
        self.transfers_context.add_token(session_id, cancellation_signal.clone()).await;
        let _drop_guard = cancellation_signal.drop_guard();

        log::info!(
            "Thumbnails info {:?}",
            session.resources.iter().map(|r| r.thumbnail_path.clone()).collect::<Vec<_>>()
        );

        let mut resource_rx = self.inbound_data_stream_receiver.retrieve_timed(Duration::from_secs(11)).await?;
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

                    log::info!("Found start delimiter {start_delimiter:?}");

                    let Some(resource_index) = thumbnail_paths.iter().position(|it| it.0 == start_delimiter.resource_id) else {
                        return Err(WebRtcErrors::InvalidDelimiter(format!(
                            "The first delimiter is not match with any resource {start_delimiter:?}"
                        )));
                    };

                    let resource_path = thumbnail_paths.swap_remove(resource_index).1;

                    let mut writer = repo
                        .write(resource_path.clone(), start_delimiter.compressed)
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
                        resource_id: start_delimiter.resource_id,
                        path: resource_path,
                    };

                    let _ = core_request.response(TransferOperationOutput::ThumbnailUpdated(event)).await;
                }

                Ok(thumbnail_paths)
            })
        };

        while !session.is_completed() {
            let start_delimiter = TransferDelimiterShema::forward_to_next_resource(&mut resource_rx, session_id)
                .with_cancel(&cancellation_signal)
                .await??;

            let Some((resource_path, resource_size)) = session
                .resources
                .iter()
                .find(|it| it.order_id == start_delimiter.resource_id)
                .map(|it| (it.path.clone(), it.size))
            else {
                return Err(WebRtcErrors::InvalidDelimiter(format!(
                    "The first delimiter is not match with any resource {start_delimiter:?}"
                )));
            };

            let mut writer = self.resource_repo.write(resource_path.clone(), start_delimiter.compressed).await?;

            let Some(progress_update) = session.resource_mut_progress(start_delimiter.resource_id) else {
                return Err(anyhow!("Missing progress for resource {}", start_delimiter.resource_id).into());
            };

            let mut total_written_bytes = 0u64;
            log::info!("Begin downloading resource {:?} {}", resource_path, resource_size);
            while let Ok(Some(packet)) = resource_rx.next().with_cancel(&cancellation_signal).await {
                if TransferDelimiterShema::from_end_packet(&packet, session_id).is_ok() {
                    progress_update.success();
                    break;
                }

                let written_bytes = packet.len() as u64;
                writer
                    .write(packet.to_vec().into())
                    .with_cancel(&cancellation_signal)
                    .await?
                    .map_err(|it| WebRtcErrors::PersistentError(PersistenceError::IOError(format!("{it:?}"))))?;
                total_written_bytes += written_bytes;
                progress_update.update_progress(written_bytes);
                core_request.response_throttle(TransferResourceProgressUpdate(progress_update.clone())).await;
            }

            log::info!("Complete downloading resource {:?} len {total_written_bytes}", resource_path);
            writer.end().await?;
            progress_update.complete();
            let _ = core_request.response(TransferResourceProgressUpdate(progress_update.clone())).await;
        }

        // Giving max 10s more for thumbnail to complete
        drop(resource_rx);
        cancellation_signal.cancel_after(Duration::from_secs(10));
        let _ = thumbnail_handle.await;

        Ok(session.status())
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

                    let begin_delimiter = TransferDelimiterShema::new(session_id, id, true, false).as_bytes()?;

                    if let Err(e) = thumbnail_channel.unbounded_send((peer_id, begin_delimiter)) {
                        log::error!("Failed to send delimiter to peer for thumbnail {peer_id:?}: {e:?}");
                        return Err(WebRtcErrors::PersistentError(PersistenceError::IOError(format!("{e:?}"))));
                    }

                    while let Ok(Ok(Some(bytes))) = reader.next(None).with_cancel(&thumbnail_cancel_signal).await {
                        let bytes = Packet::from(bytes);
                        if !bytes.is_empty() {
                            let _ = thumbnail_channel.unbounded_send((peer_id, bytes));
                        }

                        if buffer.buffered_amount(TRANSFER_THUMBNAIL_CHANNEL_ID).await > MAX_BUFFER_SIZE {
                            buffer.flush_timeout(TRANSFER_THUMBNAIL_CHANNEL_ID).await?;
                        }
                    }

                    let end_delimiter = TransferDelimiterShema::new(session_id, id, false, false).as_bytes()?;
                    if let Err(e) = thumbnail_channel.unbounded_send((peer_id, end_delimiter)) {
                        log::error!("Failed to send delimiter to peer for thumbnail {peer_id:?}: {e:?}");
                        return Err(WebRtcErrors::PersistentError(PersistenceError::IOError(format!("{e:?}"))));
                    }
                }

                Ok(session_thumbnail_paths)
            })
        };

        let resource_cancel_signal = cancellation_signal.clone();
        while !session.is_completed() {
            let Some((resource_path, order_id, size, name)) = session
                .get_next_transfer_resource()
                .map(|it| (it.path.clone(), it.order_id, it.size, it.name.clone()))
            else {
                break;
            };

            let is_compressed = is_compressible(&name);
            let chunk_size = 254 * 1024u64;
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

            let delimiter = TransferDelimiterShema::start(session_id, order_id, is_compressed).as_bytes()?;
            if let Err(e) = self.reliable_data_channel.unbounded_send((peer_id, delimiter)) {
                let msg = format!("Failed to send delimiter to peer {peer_id:?}: {e:?}");
                progress_update.fail(msg);
                let _ = core_request.response_throttle(TransferResourceProgressUpdate(progress_update.clone())).await;
                continue;
            }

            let (queue_tx, mut queue_rx) = mpsc::unbounded();
            let mut buff_counter = 0;
            let time_to_drain = Duration::from_secs(5);
            let mut drain_tick = Instant::now();

            let mut feedback_receiver = self.transfer_feedback_receiver.retrieve().await?;
            loop {
                let mut queue_fut = queue_rx.next().fuse();
                let mut reader_fut = reader.c_next(Some(chunk_size)).with_cancel(&resource_cancel_signal).fuse();
                let mut fb_fut = feedback_receiver.next().with_cancel(&resource_cancel_signal).fuse();
                let (bytes, raw_size, feedback) = select_biased! {
                    q = queue_fut => {
                        q.map(|it| (Some(it.0), Some(it.1), None)).unwrap_or((None, None, None))
                    },
                    r = reader_fut => {
                        let res = r??;
                        res.map(|it| (Some(it.0), Some(it.1), None)).unwrap_or((None, None, None))
                    },
                    fb = fb_fut => {
                        fb.map(|f| (None, None, Some(f))).unwrap_or((None, None, None))
                    },
                    complete => (None, None, None),
                };

                let action = match (bytes, raw_size, feedback) {
                    (Some(bytes), Some(raw_size), None) => {
                        let action = self.fec_sender.lock().await.send(bytes)?;
                        buff_counter += bytes.len();
                        progress_update.update_progress(raw_size as u64);
                        let _ = core_request.response_throttle(TransferResourceProgressUpdate(progress_update.clone())).await;
                        action
                    }
                    (None, Some(_), Some(fb)) => self.fec_sender.lock().await.feedback(fb)?,
                    _ => {
                        break;
                    }
                };

                match action {
                    FecAction::Framed(frames) => {
                        for frame in frames {
                            let _ = self.unreliable_data_channel.unbounded_send((self.peer.peer_id(), frame.serialize()));
                        }
                    }
                    FecAction::Retransmit(frames) => {
                        for frame in frames {
                            let _ = self.reliable_data_channel.unbounded_send((self.peer.peer_id(), frame.serialize()));
                        }
                    }
                    FecAction::Terminated => {
                        log::info!("Fec sender terminated, aborting resource transfer");
                        break;
                    }
                    FecAction::Noop | FecAction::Feedback(_) | FecAction::Constructed(_) => {
                        // Will not happens, do nothing
                    }
                };

                if buff_counter > MAX_BUFFER_SIZE {
                    buff_counter = self.buffer.buffered_amount(TRANSFER_RESOURCE_UNRELIABLE_CHANNEL_ID).await;
                    if buff_counter > MAX_BUFFER_SIZE / 3 || drain_tick.elapsed() > time_to_drain {
                        drain_tick = Instant::now();
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
                            let flushed = self.buffer.flush_timeout(TRANSFER_RESOURCE_UNRELIABLE_CHANNEL_ID).await?;
                            let time = tick.elapsed().as_secs_f64().max(f64::MIN);
                            let new_stats = self
                                .buffer
                                .channel_bytes_sent_received(TRANSFER_RESOURCE_UNRELIABLE_CHANNEL_ID)
                                .await
                                .map(|it| it.0)
                                .unwrap_or(0);
                            let bw = ((new_stats - stats).max(flushed)) as f64 / time;
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
                        send?;
                        reader.compression_stats_mut().update_network_bandwidth(bw);
                    }
                }
            }

            let end_delimiter = TransferDelimiterShema::end(session_id, order_id, is_compressed).as_bytes()?;
            if let Err(e) = self.reliable_data_channel.unbounded_send((peer_id, end_delimiter)) {
                let msg = format!("Failed to send delimiter to peer {peer_id:?}: {e:?}");
                progress_update.fail(msg);
                let _ = core_request.response(TransferResourceProgressUpdate(progress_update.clone()));
                continue;
            }
            progress_update.complete();
            let _ = core_request.response(TransferResourceProgressUpdate(progress_update.clone())).await;

            log::info!(
                "Complete transferring resource {resource_path:?} with status {:?} total_sent {:?}",
                progress_update.status,
                total_sent_bytes
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
