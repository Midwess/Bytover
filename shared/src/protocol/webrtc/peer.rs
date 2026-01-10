use crate::app::operations::p2p::P2POperationOutput;
use crate::app::operations::transfer::TransferOperationOutput;
use crate::app::operations::CoreOperationOutput;
use crate::entities::local_resource::{LocalResource, LocalResourcePath, ResourceType};
use crate::entities::peer::Peer as PeerEntity;
use crate::entities::transfer_session::TransferProgress;
use crate::errors::CoreError;
use crate::protocol::webrtc::errors::WebRtcErrors;
use crate::protocol::webrtc::fec::{FecAction, FecReceiver, FecSender, Frame, CHUNK_SIZE, DATA_SHARDS_DEFAULT};
use crate::protocol::webrtc::message_channel::DirectMessageChannel;
use crate::protocol::webrtc::quad_channel::QuadUnreliableChannel;
use crate::protocol::webrtc::transfer::{TransferDelimiterShema, TransfersContext};
use crate::protocol::webrtc::webrtc::{MAX_BUFFER_SIZE, MAX_NUM_BLOCK, MIN_BUFFER_SIZE, TRANSFER_RESOURCE_RELIABLE_CHANNEL_ID};
use crate::repository::local_resource::LocalResourceRepository;
use crate::repository::transfer_session::TransferSessionRepository;
use crate::shell::api::CoreRequest;
use crate::utils::compression::is_compressible;
use anyhow::anyhow;
use bytes::Bytes;
use core_services::utils::cancellation::{CancellationToken, FutureExtension};
use core_services::utils::yield_container::YieldContainer;
use futures::channel::mpsc;
use futures::channel::mpsc::unbounded;
use futures_util::lock::Mutex;
use futures_util::{select, select_biased, FutureExt, SinkExt, StreamExt};
use matchbox_protocol::PeerId;
use matchbox_socket::{Packet, PeerBuffered};
use n0_future::time::{sleep, Instant};
use once_cell::sync::OnceCell;
use schema::devlog::bitbridge::fec_feedback::Feedback;
use schema::devlog::bitbridge::peer_message_body::Response::IntroduceResponse;
use schema::devlog::bitbridge::peer_message_body::{Request, Response};
use schema::devlog::bitbridge::{
    DownloadResourceRequest,
    IntroduceRequestMessage,
    IntroduceResponseMessage,
    P2pCancelSessionRequest,
    P2pTransferSessionMessage,
    PeerErrorsMessage,
    PeerMessage,
    ResourceTypeMessage,
    ViewSessionDetailRequest,
    ViewSessionDetailResponse,
    VoidResponseMessage
};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU16, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

// This configuration is what I found the best
// Increasing these number could cause randomly hang
// Maybe the NAT got overloaded
// We need to figure out why.
const ON_HOLD_STOP_THRESHOLD: u8 = 3;

pub struct WebRtcPeer {
    pub peer: PeerEntity,
    pub resource_repo: Arc<dyn LocalResourceRepository>,
    pub transfer_session_repo: Arc<dyn TransferSessionRepository>,

    pub msg_channel: DirectMessageChannel,
    pub unordered_msg_channel: DirectMessageChannel,
    pub quad_unreliable_channel: Arc<QuadUnreliableChannel>,
    pub reliable_data_channel: mpsc::UnboundedSender<(PeerId, Packet)>,
    pub buffer: PeerBuffered,

    pub transfer_feedback_receiver: YieldContainer<mpsc::UnboundedReceiver<Feedback>>,
    pub transfer_feedback_sender: mpsc::UnboundedSender<Feedback>,

    pub transfers_context: TransfersContext,

    pub inbound_data_stream_receiver: YieldContainer<mpsc::UnboundedReceiver<Packet>>,
    pub inbound_data_stream_sender: mpsc::UnboundedSender<Packet>,

    pub outbound_packet_receiver: YieldContainer<mpsc::Receiver<(u16, Packet, bool)>>,
    pub outbound_packet_sender: mpsc::Sender<(u16, Packet, bool)>,

    pub prefix_channels: Mutex<HashMap<u16, mpsc::Sender<Packet>>>,

    pub bandwidth: Arc<AtomicU64>,

    pub core_request: OnceCell<CoreRequest>
}

impl WebRtcPeer {
    pub async fn new(
        user: PeerEntity,
        msg_channel: DirectMessageChannel,
        unordered_msg_channel: DirectMessageChannel,
        reliable_data_channel: mpsc::UnboundedSender<(PeerId, Packet)>,
        quad_unreliable_channel: QuadUnreliableChannel,
        buffer: PeerBuffered,
        repository: Arc<dyn LocalResourceRepository>,
        transfer_session_repo: Arc<dyn TransferSessionRepository>
    ) -> Result<Self, WebRtcErrors> {
        let (transfer_feedback_sender, transfer_feedback_receiver) = unbounded();

        let (data_tx, data_rx) = unbounded();
        let (outbound_packet_tx, outbound_packet_rx) = mpsc::channel(16);

        let introduce_request = IntroduceRequestMessage {
            mine: PeerMessage {
                peer_id: user.id().to_string(),
                name: user.name.clone(),
                avatar_url: user.avatar_url.clone(),
                device: user.device.clone().into(),
                email: user.email.clone()
            }
        };

        log::info!("Sending introduce request to other peer {:?}", introduce_request.mine.peer_id);
        let IntroduceResponse(response) = msg_channel.send(Request::IntroduceRequest(introduce_request), None).await? else {
            return Err(WebRtcErrors::FailedToIntroducePeer);
        };

        log::info!("Received introduce response from other peer {:?}", response.peer.peer_id);

        let peer: PeerEntity = response.peer.into();

        Ok(Self {
            msg_channel,
            unordered_msg_channel,
            peer,
            transfer_feedback_receiver: YieldContainer::new(transfer_feedback_receiver),
            transfer_feedback_sender,
            reliable_data_channel,
            quad_unreliable_channel: Arc::new(quad_unreliable_channel),
            transfers_context: TransfersContext::new(),
            resource_repo: repository,
            transfer_session_repo,
            inbound_data_stream_sender: data_tx,
            inbound_data_stream_receiver: YieldContainer::new(data_rx),
            outbound_packet_receiver: YieldContainer::new(outbound_packet_rx),
            outbound_packet_sender: outbound_packet_tx,
            prefix_channels: Mutex::new(HashMap::new()),
            bandwidth: Arc::new(AtomicU64::new(0)),
            buffer,
            core_request: Default::default()
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
        unordered_msg_channel: DirectMessageChannel,
        reliable_data_channel: mpsc::UnboundedSender<(PeerId, Packet)>,
        quad_unreliable_channel: QuadUnreliableChannel,
        buffer: PeerBuffered,
        repository: Arc<dyn LocalResourceRepository>,
        transfer_session_repo: Arc<dyn TransferSessionRepository>
    ) -> Result<Self, WebRtcErrors> {
        log::info!("Received introduce request from other peer {:?}", msg.mine.peer_id);
        let (transfer_feedback_sender, transfer_feedback_receiver) = unbounded();
        let (data_tx, data_rx) = unbounded();
        let (outbound_packet_tx, outbound_packet_rx) = mpsc::channel(16);
        let introduce_response = IntroduceResponse(IntroduceResponseMessage {
            peer: PeerMessage {
                peer_id: user.id().to_string(),
                name: user.name.clone(),
                avatar_url: user.avatar_url.clone(),
                device: user.device.clone().into(),
                email: user.email.clone()
            }
        });

        msg_channel.send_response(request_id, introduce_response).await?;
        log::info!("Sent introduce response to other peer {:?}", msg.mine.peer_id);

        let peer: PeerEntity = msg.mine.into();

        Ok(Self {
            msg_channel,
            unordered_msg_channel,
            transfer_feedback_sender,
            transfer_feedback_receiver: YieldContainer::new(transfer_feedback_receiver),
            peer,
            reliable_data_channel,
            quad_unreliable_channel: Arc::new(quad_unreliable_channel),
            transfers_context: TransfersContext::new(),
            resource_repo: repository,
            transfer_session_repo,
            inbound_data_stream_sender: data_tx,
            inbound_data_stream_receiver: YieldContainer::new(data_rx),
            outbound_packet_receiver: YieldContainer::new(outbound_packet_rx),
            outbound_packet_sender: outbound_packet_tx,
            prefix_channels: Mutex::new(HashMap::new()),
            bandwidth: Arc::new(AtomicU64::new(0)),
            buffer,
            core_request: Default::default()
        })
    }

    pub fn start_core_stream(&self, core_request: CoreRequest) {
        let _ = self.core_request.set(core_request);
    }

    pub async fn process_message_packet(&self, request_id: String, msg: Request) {
        match msg {
            Request::CancelRequest(request) => {
                log::info!("Received cancel request {:?}", request);
                if let Some(resource_id) = request.resource_id {
                    self.transfers_context.cancel_resource(request.session_id, resource_id).await;
                } else {
                    self.transfers_context.cancel_transfer(request.session_id).await;
                }
            }
            Request::ViewSessionRequest(req) => {
                log::info!("Received view session request for order_id {}", req.order_id);
                let response = CoreOperationOutput::P2P(P2POperationOutput::ReceivedViewSessionRequest {
                    peer_id: self.peer.id().to_string(),
                    request_id,
                    order_id: req.order_id,
                    password: req.password
                });

                if let Some(core_request) = self.core_request() {
                    log::info!("Forwarding to core");
                    core_request.response(response).await;
                }
            }
            Request::DownloadResourceRequest(req) => {
                let response = CoreOperationOutput::P2P(P2POperationOutput::ReceivedDownloadRequest {
                    peer_id: self.peer.id().to_string(),
                    session_order_id: req.session_order_id,
                    resource_order_id: req.resource_order_id,
                    transfer_id: req.transfer_id as u16
                });
                if let Some(core_request) = self.core_request() {
                    core_request.response(response).await;
                }
            }
            Request::ResourceNotification(notification) => {
                let session_order_id = notification.session_order_id;
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
                        let response = CoreOperationOutput::P2P(P2POperationOutput::ReceivedResourceNotification {
                            session_order_id,
                            resource,
                            peer_id: self.peer.id().to_string()
                        });

                        core_request.response(response).await;
                    }
                }

                let _ = self.msg_channel.send_response(request_id, Response::VoidResponse(VoidResponseMessage {})).await;
            }
            Request::FecFeedback(feedback) => {
                if let Some(feedback) = feedback.feedback {
                    let _ = self.transfer_feedback_sender.unbounded_send(feedback);
                };
            }
            _ => {}
        }
    }

    pub async fn process_data_packet(&self, packet: Packet) {
        let _ = self.inbound_data_stream_sender.unbounded_send(packet);
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
        let cancel_msg = P2pCancelSessionRequest {
            session_id,
            resource_id: None
        };

        self.transfers_context.cancel_transfer(session_id).await;

        log::info!("Cancelling transfer session {session_id} to peer {}", self.peer.peer_id());
        let request = Request::CancelRequest(cancel_msg);
        let _ = self.msg_channel.notify(request).await;
    }

    pub async fn cancel_resource_transfer(&self, session_id: u64, resource_id: u64) {
        let cancel_msg = P2pCancelSessionRequest {
            session_id,
            resource_id: Some(resource_id)
        };

        self.transfers_context.cancel_resource(session_id, resource_id).await;

        log::info!(
            "Cancelling resource {resource_id} in session {session_id} to peer {}",
            self.peer.peer_id()
        );
        let request = Request::CancelRequest(cancel_msg);
        let _ = self.msg_channel.notify(request).await;
    }

    #[allow(dead_code)]
    async fn handle_fec_action(&self, action: FecAction) -> Result<(Vec<Packet>, Option<Instant>), WebRtcErrors> {
        match action {
            FecAction::Constructed(packets_with_prefix, next_check) => {
                let packets = packets_with_prefix.into_iter().map(|(_, packet)| packet).collect();
                Ok((packets, Some(next_check)))
            }
            FecAction::Feedback(fb, next_check) => {
                log::info!("Sending FEC feedback: {:?}", fb);
                self.unordered_msg_channel.notify(Request::FecFeedback(fb)).await?;
                Ok((vec![], Some(next_check)))
            }
            FecAction::Terminated => {
                log::warn!("FEC terminated");
                Err(WebRtcErrors::InvalidDelimiter("FEC terminated".into()))
            }
            FecAction::Queued(time) => Ok((vec![], Some(time))),
            FecAction::Noop => Ok((vec![], None)),
            _ => Ok((vec![], None))
        }
    }

    pub async fn cancel_transfer_session(&self, session_id: u64) -> Result<(), WebRtcErrors> {
        self.transfers_context.cancel_transfer(session_id).await;
        self.msg_channel
            .notify(Request::CancelRequest(P2pCancelSessionRequest {
                session_id,
                resource_id: None
            }))
            .await?;
        Ok(())
    }

    pub async fn sending_loop(&self) -> Result<(), WebRtcErrors> {
        let mut fec_sender = FecSender::new(self.peer.peer_id(), 1024);
        let mut feedback_receiver = self.transfer_feedback_receiver.retrieve().await?;
        let mut packet_rx = self.outbound_packet_receiver.retrieve().await?;
        let mut buff_counter = MAX_BUFFER_SIZE - CHUNK_SIZE;
        let mut hold_counter: u8 = 0;
        let mut block_holding_id = fec_sender.block_id;

        loop {
            if let Some(rtt) = self.buffer.rtt().await {
                fec_sender.set_rtt(rtt as u64);
            }

            let (packet, feedback) = {
                if hold_counter >= ON_HOLD_STOP_THRESHOLD {
                    log::debug!("Stopped {}", fec_sender.block_id);
                    let timeout_fut = sleep(Duration::from_secs(60 * 100)).fuse();
                    let fb_fut = feedback_receiver.next().fuse();

                    futures::pin_mut!(timeout_fut);
                    futures::pin_mut!(fb_fut);
                    select_biased! {
                        _ = timeout_fut => {
                            return Err(anyhow!("Timeout waiting for end acknowledgment").into());
                        },
                        fb = fb_fut => {
                            let fb = fb;
                            fb.map(|f| (None, Some(f))).unwrap_or((None, None))
                        },
                    }
                } else {
                    let reader_fut = packet_rx.next().fuse();
                    let fb_fut = feedback_receiver.next().fuse();

                    futures::pin_mut!(fb_fut);
                    futures::pin_mut!(reader_fut);
                    select_biased! {
                        r = reader_fut => {
                            r.map(|it| (Some(it), None)).unwrap_or((None, None))
                        },
                        fb = fb_fut => {
                            let fb = fb;
                            fb.map(|f| (None, Some(f))).unwrap_or((None, None))
                        },
                    }
                }
            };

            let (reliable, action) = match (packet, feedback) {
                (Some((prefix, packet, reliable)), _) => (reliable, fec_sender.send(prefix, packet)?),
                (_, Some(fb)) => {
                    use schema::devlog::bitbridge::fec_feedback::Feedback;
                    if let Feedback::Network(stats) = fb {
                        if let Some(peer_block_id) = stats.current_block_id {
                            if block_holding_id <= peer_block_id {
                                let diff = peer_block_id.abs_diff(fec_sender.block_id);
                                log::debug!("Received network report {diff}");
                                if diff == 0 {
                                    hold_counter = 0;
                                } else {
                                    hold_counter = hold_counter.saturating_sub(1);
                                }

                                if diff >= ON_HOLD_STOP_THRESHOLD as u32 * MAX_NUM_BLOCK as u32 {
                                    hold_counter = ON_HOLD_STOP_THRESHOLD;
                                    buff_counter = MAX_BUFFER_SIZE;
                                }
                            }
                        }

                        if hold_counter == 0 {
                            block_holding_id = fec_sender.block_id;
                        }
                    }

                    (true, fec_sender.feedback(fb))
                }
                _ => break
            };

            match action {
                FecAction::Framed(frames) => {
                    for frame in frames {
                        let packet = frame.serialize();
                        buff_counter += packet.len();
                        if reliable {
                            let _ = self.reliable_data_channel.unbounded_send((self.peer.peer_id(), packet));
                        } else {
                            let _ = self.quad_unreliable_channel.send(self.peer.peer_id(), packet);
                        }
                    }
                }
                FecAction::Retransmit(frames) => {
                    let frame_idx = frames.iter().map(|it| it.frame_idx).collect::<Vec<_>>();
                    log::info!("Retransmit {:?} {:?}", frames.first().map(|it| it.block_id), frame_idx);
                    for frame in frames {
                        let packet = frame.serialize();
                        buff_counter += packet.len();
                        let _ = self.reliable_data_channel.unbounded_send((self.peer.peer_id(), packet));
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
                let stats_before = self.quad_unreliable_channel.bytes_sent().await;
                let timeout = Duration::from_millis(20 * fec_sender.rtt().max(100));

                self.quad_unreliable_channel.wait_buffer_low(MIN_BUFFER_SIZE, timeout).await;
                self.buffer.wait_buffer_low(TRANSFER_RESOURCE_RELIABLE_CHANNEL_ID, MIN_BUFFER_SIZE, timeout).await;
                buff_counter = MIN_BUFFER_SIZE;

                hold_counter += 1;
                hold_counter = hold_counter.min(ON_HOLD_STOP_THRESHOLD);
                let hold_delimiter = TransferDelimiterShema::hold(hold_counter).as_bytes()?;
                let FecAction::Framed(frames) = fec_sender.send(0, hold_delimiter.to_vec().into_boxed_slice())? else {
                    return Err(anyhow!("Failed to build hold delimiter").into());
                };

                for frame in frames {
                    if hold_counter >= ON_HOLD_STOP_THRESHOLD {
                        let _ = self.reliable_data_channel.unbounded_send((self.peer.peer_id(), frame.serialize()));
                    } else {
                        let _ = self.quad_unreliable_channel.send(self.peer.peer_id(), frame.serialize());
                    }
                }

                let time = tick.elapsed().as_secs_f64().max(f64::MIN);
                let stats_after = self.quad_unreliable_channel.bytes_sent().await;
                let total_sent = stats_after.saturating_sub(stats_before);
                let bw = (total_sent as f64 / time) as u64;
                let _bw_kbps = bw / 1000;
            }
        }

        Ok(())
    }

    pub async fn request_session_detail(
        &self,
        core_request: CoreRequest,
        order_id: u64,
        password: Option<String>
    ) -> Result<(), WebRtcErrors> {
        use schema::devlog::bitbridge::view_session_detail_response::Result as ResponseResult;

        log::info!("Requesting session detail for order_id {}", order_id);

        let request = ViewSessionDetailRequest { order_id, password };

        let timeout_token = CancellationToken::timeout(Duration::from_secs(60));

        let response_result = self
            .msg_channel
            .send(Request::ViewSessionRequest(request), None)
            .with_cancel(&timeout_token)
            .await
            .map_err(|_| {
                log::error!("Timeout waiting for session detail response");
                WebRtcErrors::Timeout
            })??;

        match response_result {
            Response::ViewSessionResponse(resp) => match resp.result {
                Some(ResponseResult::Session(session)) => {
                    core_request
                        .response(CoreOperationOutput::Transfer(TransferOperationOutput::SessionDetailReceived(
                            session
                        )))
                        .await;
                }
                Some(ResponseResult::Error(error_type)) => {
                    let error_msg = PeerErrorsMessage::try_from(error_type).unwrap_or(PeerErrorsMessage::InvalidRequest);
                    core_request.response(CoreOperationOutput::Error(CoreError::PeerRequestError(error_msg))).await;
                    return Err(WebRtcErrors::PeerError(error_msg.to_string()));
                }
                _ => {
                    return Err(WebRtcErrors::InvalidResponse("Unexpected response".to_string()));
                }
            },
            _ => {
                return Err(WebRtcErrors::InvalidResponse("Expected ViewSessionResponse".to_string()));
            }
        }

        Ok(())
    }

    pub async fn send_session_detail_response(
        &self,
        request_id: String,
        session_message: Option<P2pTransferSessionMessage>,
        resources: Option<Vec<LocalResource>>,
        error: Option<CoreError>
    ) -> Result<(), WebRtcErrors> {
        log::info!("Sending session detail response");
        use schema::devlog::bitbridge::view_session_detail_response::Result as ResponseResult;

        if let Some(error_msg) = error {
            log::error!("Failed to send session detail response: {:?}", error_msg);
            match error_msg {
                CoreError::PeerRequestError(e) => {
                    self.msg_channel
                        .send_response(
                            request_id,
                            Response::ViewSessionResponse(ViewSessionDetailResponse {
                                result: Some(ResponseResult::Error(e.into()))
                            })
                        )
                        .await?;
                }
                e => {
                    log::error!("Failed to send session detail response: {:?}", e);
                    self.msg_channel
                        .send_response(
                            request_id,
                            Response::ViewSessionResponse(ViewSessionDetailResponse {
                                result: Some(ResponseResult::Error(PeerErrorsMessage::InvalidRequest.into()))
                            })
                        )
                        .await?;
                }
            }
            return Ok(());
        }

        let Some(proto_session) = session_message else { return Ok(()) };

        log::info!(
            "Sending session detail: order_id={}, password_protected={}, has_resources={}",
            proto_session.order_id,
            proto_session.password_protected,
            resources.is_some()
        );

        let response = ViewSessionDetailResponse {
            result: Some(ResponseResult::Session(proto_session.clone()))
        };

        self.msg_channel.send_response(request_id, Response::ViewSessionResponse(response)).await?;

        sleep(Duration::from_millis(100)).await;

        if let Some(resources) = resources {
            if !resources.is_empty() {
                let session_order_id = proto_session.order_id;
                for resource in resources {
                    self.send_resource_notification(session_order_id, resource).await?;
                    sleep(Duration::from_millis(20)).await;
                }
            }
        } else {
            log::info!(
                "No resources to send for session {} (password-protected, awaiting auth)",
                proto_session.order_id
            );
        }

        Ok(())
    }

    pub async fn send_resource_notification(&self, session_order_id: u64, resource: LocalResource) -> Result<(), WebRtcErrors> {
        use schema::devlog::bitbridge::ResourceNotificationRequest;

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

        let _ = self.msg_channel.send(Request::ResourceNotification(notification), None).await?;
        Ok(())
    }

    pub async fn request_resource_download(
        &self,
        core_request: CoreRequest,
        session_order_id: u64,
        resource: LocalResource,
        mut progress: TransferProgress
    ) -> Result<(), WebRtcErrors> {
        static TRANSFER_ID_COUNTER: AtomicU16 = AtomicU16::new(1);

        let resource_order_id = resource.order_id;

        let transfer_id = TRANSFER_ID_COUNTER.fetch_add(1, Ordering::SeqCst);

        let request = DownloadResourceRequest {
            session_order_id,
            resource_order_id,
            transfer_id: transfer_id as u32
        };

        let (tx, mut rx) = mpsc::channel::<Packet>(64);
        let prefix = transfer_id;
        self.prefix_channels.lock().await.insert(prefix, tx);

        let resource_token = self.transfers_context.get_or_create_resource_token(session_order_id, resource_order_id).await;

        log::info!(
            "Requesting download for resource {:?}, registered prefix channel: {}",
            request,
            prefix
        );

        progress.update_progress(1);
        core_request
            .response(TransferOperationOutput::TransferResourceProgressUpdate(progress.clone()))
            .await;

        self.msg_channel.notify(Request::DownloadResourceRequest(request)).await?;
        let resource_repo = self.resource_repo.clone();

        let start_delimiter = loop {
            match rx.next().with_cancel(&resource_token).await? {
                Some(packet) => {
                    if let Ok(v) = TransferDelimiterShema::from_start_packet(&packet, session_order_id) {
                        break v
                    }
                }
                None => {
                    log::warn!("Channel closed before receiving start delimiter");
                    return Err(WebRtcErrors::InvalidDelimiter("Channel closed before start delimiter".into()));
                }
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

        let mut total_packet = 0;
        loop {
            log::debug!(
                "Waiting for packet {} on prefix {} channel (resource {})",
                total_packet + 1,
                prefix,
                resource_id
            );
            match rx.next().with_cancel(&resource_token).await? {
                Some(packet) => {
                    total_packet += 1;
                    log::debug!(
                        "Received packet {} (size={}) for resource {}",
                        total_packet,
                        packet.len(),
                        resource_id
                    );
                    if TransferDelimiterShema::from_end_packet(&packet, session_order_id).is_ok() {
                        log::info!("Received end delimiter for resource {}", resource_id);
                        progress.success();
                        core_request
                            .response(TransferOperationOutput::TransferResourceProgressUpdate(progress.clone()))
                            .await;
                        break;
                    }

                    let bytes = Bytes::from(packet.to_vec());
                    let Some(written) = writer.d_write(bytes).await? else { continue };

                    progress.update_progress(written as u64);
                    core_request
                        .response_throttle(TransferOperationOutput::TransferResourceProgressUpdate(progress.clone()))
                        .await;
                }
                None => {
                    log::error!(
                        "Channel closed before end delimiter. Received {} packets for resource {}, prefix {}",
                        total_packet,
                        resource_id,
                        prefix
                    );
                    return Err(WebRtcErrors::InvalidDelimiter("Channel closed before end delimiter".into()));
                }
            }
        }

        self.prefix_channels.lock().await.remove(&prefix);

        log::info!("Completed download for resource {}", resource_id);

        Ok(())
    }

    pub async fn download_all_resources(
        &self,
        core_request: CoreRequest,
        session_order_id: u64,
        session_resource: LocalResource,
        resources: Vec<LocalResource>
    ) -> Result<(), WebRtcErrors> {
        use crate::entities::transfer_session::TransferType;

        log::info!("Starting download all resources for session {}", session_order_id);

        let token = self
            .transfers_context
            .get_or_create_resource_token(session_order_id, session_resource.order_id)
            .await;
        let zip_path = session_resource.path.clone();

        if let Err(e) = self.transfer_session_repo.start_download_session(zip_path.clone()).await {
            log::error!("Failed to start download session: {:?}", e);
            self.cancel_resource_transfer(session_order_id, session_resource.order_id).await;
            return Err(WebRtcErrors::InvalidDelimiter(format!(
                "Failed to start download session: {:?}",
                e
            )));
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
                    log::warn!(
                        "Download all cancelled for session {}, resource {}",
                        session_order_id,
                        resource_id
                    );
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

        if let Err(e) = self.transfer_session_repo.stop_download_session(zip_path).await {
            log::error!("Failed to stop download session: {:?}", e);
        }

        if download_failed {
            log::info!("Download all resources failed for session {}", session_order_id);
            self.cancel_resource_transfer(session_order_id, session_resource.order_id).await;
            return Err(WebRtcErrors::InvalidDelimiter("Download all failed".into()));
        }

        log::info!("Completed download all resources for session {}", session_order_id);

        Ok(())
    }

    pub async fn stream_resource(&self, session_id: u64, transfer_id: u16, resource: LocalResource) -> Result<(), WebRtcErrors> {
        let resource_id = resource.order_id;
        let prefix = transfer_id;

        let resource_token = self.transfers_context.get_or_create_resource_token(session_id, resource_id).await;

        let resource_name = match resource.r#type {
            ResourceType::Folder => format!("{}.zip", &resource.name),
            _ => resource.name.clone()
        };

        log::info!("Streaming resource {} for transfer id {}", resource_id, transfer_id);
        let compressed = is_compressible(&resource_name);

        let start_delimiter = TransferDelimiterShema::start(session_id, resource_id, compressed);
        let start_packet = start_delimiter.as_bytes()?;
        let mut outbound_packet_sender = self.outbound_packet_sender.clone();
        match outbound_packet_sender.send((prefix, start_packet, true)).with_cancel(&resource_token).await {
            Ok(Ok(_)) => {}
            Ok(Err(e)) => {
                return Err(WebRtcErrors::InvalidDelimiter(format!(
                    "Failed to send start delimiter: {:?}",
                    e
                )))
            }
            Err(_) => {
                log::info!("Stream cancelled while sending start delimiter");
                return Err(WebRtcErrors::InvalidDelimiter("Stream cancelled".into()));
            }
        }

        let chunk_size = if compressed {
            (CHUNK_SIZE * DATA_SHARDS_DEFAULT - CHUNK_SIZE) as u64
        } else {
            (CHUNK_SIZE * DATA_SHARDS_DEFAULT) as u64
        };

        let mut cursor = self.resource_repo.read(resource.path.clone(), chunk_size as usize, compressed).await?;

        loop {
            cursor
                .compression_stats_mut()
                .update_network_bandwidth(self.bandwidth.load(Ordering::Relaxed) as f64 * 1024f64);
            match cursor.c_next(None).await? {
                Some((data, _raw_size)) => {
                    if data.is_empty() {
                        log::warn!("Cursor return empty data");
                        break;
                    }

                    let packet = data.to_vec().into_boxed_slice();
                    match outbound_packet_sender.send((prefix, packet, false)).with_cancel(&resource_token).await {
                        Ok(Ok(_)) => {}
                        Ok(Err(e)) => return Err(anyhow!("Failed to send data packet: {:?}", e).into()),
                        Err(_) => {
                            log::info!("Stream cancelled while sending data packet");
                            return Err(WebRtcErrors::InvalidDelimiter("Stream cancelled".into()));
                        }
                    }
                }
                None => {
                    break;
                }
            }
        }

        let end_delimiter = TransferDelimiterShema::end(session_id, resource_id, compressed);
        let end_packet = end_delimiter.as_bytes()?;
        outbound_packet_sender
            .send((prefix, end_packet, true))
            .with_cancel(&resource_token)
            .await?
            .map_err(|it| anyhow!("Failed to send end delimiter: {:?}", it))?;

        log::info!("Completed streaming resource {} for session {}", resource_id, session_id);
        Ok(())
    }

    pub async fn receiving_loop(&self) -> Result<(), WebRtcErrors> {
        use schema::devlog::bitbridge::fec_feedback::Feedback;
        use schema::devlog::bitbridge::{FecFeedback, NetworkStats};

        let mut fec_receiver = FecReceiver::new();
        let mut data_rx = self.inbound_data_stream_receiver.retrieve().await?;
        let mut next_check_time: Option<Instant> = None;

        loop {
            let frames = {
                let mut frames = Vec::new();

                let check_time = next_check_time.take().unwrap_or(fec_receiver.calculate_next_check_time());
                let now = Instant::now();
                let timeout_duration = if check_time > now {
                    check_time.duration_since(now)
                } else {
                    Duration::from_millis(50)
                };

                let packet_result = {
                    select! {
                        packet = data_rx.next().fuse() => packet,
                        _ = sleep(timeout_duration).fuse() => None,
                    }
                };

                if let Some(packet) = packet_result {
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

            let action = if frames.is_empty() {
                fec_receiver.ping()?
            } else {
                fec_receiver.receive(frames)?
            };

            match action {
                FecAction::Constructed(packets_with_prefix, next_check) => {
                    next_check_time = Some(next_check);

                    for (prefix, packet) in packets_with_prefix {
                        if let Ok(hold) = TransferDelimiterShema::from_hold_packet(&packet) {
                            let loss_rate = fec_receiver.calculate_loss_rate();
                            let current_block_id = fec_receiver.current_block_id();
                            let rtt = self.buffer.rtt().await.unwrap_or(0.0);

                            let network_stats = NetworkStats {
                                current_block_id: Some(current_block_id),
                                rtt: Some(rtt as u32),
                                loss_rate,
                                hold_counter: hold.hold_counter().map(|it| it as u32)
                            };

                            let feedback = FecFeedback {
                                feedback: Some(Feedback::Network(network_stats))
                            };

                            let _ = self.unordered_msg_channel.notify(Request::FecFeedback(feedback)).await;
                            continue;
                        }

                        let sender = {
                            let channels = self.prefix_channels.lock().await;
                            channels.get(&prefix).cloned()
                        };

                        if let Some(mut sender) = sender {
                            log::debug!("Routing packet (size={}) to prefix {} channel", packet.len(), prefix);
                            if let Err(e) = sender.send(packet).await {
                                log::warn!(
                                    "Failed to send packet to prefix {} channel (receiver dropped): {:?}. Removing channel.",
                                    prefix,
                                    e
                                );
                                // Receiver was dropped (download canceled/errored), clean up the channel
                                self.prefix_channels.lock().await.remove(&prefix);
                            } else {
                                log::debug!("Successfully sent packet to prefix {}", prefix);
                            }
                        } else {
                            let registered_prefixes: Vec<u16> = self.prefix_channels.lock().await.keys().copied().collect();
                            log::warn!(
                                "No channel registered for prefix {}. Registered prefixes: {:?}, packet_size={}. Packet dropped.",
                                prefix,
                                registered_prefixes,
                                packet.len()
                            );
                        }
                    }
                }
                FecAction::Feedback(fb, next_check) => {
                    next_check_time = Some(next_check);
                    log::info!("Sending FEC feedback from receiver: {:?}", fb);
                    let _ = self.unordered_msg_channel.notify(Request::FecFeedback(fb)).await;
                }
                FecAction::Terminated => {
                    log::warn!("FEC receiver terminated");
                    break;
                }
                FecAction::Queued(time) => {
                    next_check_time = Some(time);
                }
                _ => {}
            }
        }

        Ok(())
    }

    pub async fn run_loop(&self) -> Result<(), WebRtcErrors> {
        let send_fut = self.sending_loop().fuse();
        let recv_fut = self.receiving_loop().fuse();
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
