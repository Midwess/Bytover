use crate::app::operations::p2p::P2POperationOutput;
use crate::app::operations::transfer::TransferOperationOutput;
use crate::app::operations::CoreOperationOutput;
use crate::entities::finding_scope::FindingScope;
use crate::entities::local_resource::{LocalResource, LocalResourcePath, ResourceType};
use crate::entities::peer::Peer as PeerEntity;
use crate::entities::transfer_session::TransferProgress;
use crate::errors::CoreError;
use crate::protocol::webrtc::errors::WebRtcErrors;
use crate::protocol::webrtc::fec::{FecAction, FecReceiver, FecSender, Frame, CHUNK_SIZE, DATA_SHARDS_DEFAULT};
use crate::protocol::webrtc::message_channel::DirectMessageChannel;
use crate::protocol::webrtc::quad_channel::QuadUnreliableChannel;
use crate::protocol::webrtc::transfer::{TransferDelimiterShema, TransfersContext};
use crate::protocol::webrtc::webrtc::{MAX_BUFFER_SIZE, MIN_BUFFER_SIZE, TRANSFER_RESOURCE_RELIABLE_CHANNEL_ID};
use crate::repository::local_resource::LocalResourceRepository;
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

static TRANSFER_ID_COUNTER: AtomicU16 = AtomicU16::new(1);
const ON_HOLD_STOP_THRESHOLD: u8 = 6;
const ON_HOLD_SLOW_THRESHOLD: u8 = 2;

pub struct WebRtcPeer {
    pub peer: PeerEntity,
    pub resource_repo: Arc<dyn LocalResourceRepository>,

    pub msg_channel: DirectMessageChannel,
    pub unordered_msg_channel: DirectMessageChannel,
    pub quad_unreliable_channel: Arc<Mutex<QuadUnreliableChannel>>,
    pub reliable_data_channel: mpsc::UnboundedSender<(PeerId, Packet)>,
    pub buffer: PeerBuffered,

    pub transfer_feedback_receiver: YieldContainer<mpsc::UnboundedReceiver<Feedback>>,
    pub transfer_feedback_sender: mpsc::UnboundedSender<Feedback>,

    pub transfers_context: TransfersContext,

    pub inbound_data_stream_receiver: YieldContainer<mpsc::UnboundedReceiver<Packet>>,
    pub inbound_data_stream_sender: mpsc::UnboundedSender<Packet>,

    pub outbound_packet_receiver: YieldContainer<mpsc::Receiver<(u16, Packet)>>,
    pub outbound_packet_sender: mpsc::Sender<(u16, Packet)>,

    pub prefix_channels: Arc<Mutex<HashMap<u16, mpsc::UnboundedSender<Packet>>>>,

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
        repository: Arc<dyn LocalResourceRepository>
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
            quad_unreliable_channel: Arc::new(Mutex::new(quad_unreliable_channel)),
            transfers_context: TransfersContext::new(),
            resource_repo: repository,
            inbound_data_stream_sender: data_tx,
            inbound_data_stream_receiver: YieldContainer::new(data_rx),
            outbound_packet_receiver: YieldContainer::new(outbound_packet_rx),
            outbound_packet_sender: outbound_packet_tx,
            prefix_channels: Arc::new(Mutex::new(HashMap::new())),
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
        repository: Arc<dyn LocalResourceRepository>
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
            quad_unreliable_channel: Arc::new(Mutex::new(quad_unreliable_channel)),
            transfers_context: TransfersContext::new(),
            resource_repo: repository,
            inbound_data_stream_sender: data_tx,
            inbound_data_stream_receiver: YieldContainer::new(data_rx),
            outbound_packet_receiver: YieldContainer::new(outbound_packet_rx),
            outbound_packet_sender: outbound_packet_tx,
            prefix_channels: Arc::new(Mutex::new(HashMap::new())),
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
                            .unwrap_or(ResourceType::File)
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

    pub async fn update_scopes(&self, scopes: Vec<String>) {
        let finding_scopes = scopes.iter().map(|s| FindingScope::new(s)).collect();
        let response = CoreOperationOutput::P2P(P2POperationOutput::PeerScopesUpdated(finding_scopes));
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

        loop {
            if let Some(rtt) = self.buffer.rtt().await {
                fec_sender.set_rtt(rtt as u64);
            }

            let (packet, feedback) = {
                if hold_counter >= ON_HOLD_STOP_THRESHOLD {
                    let timeout_fut = sleep(Duration::from_secs(60 * 10)).fuse();
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
                    let reader_fut = async {
                        let data = packet_rx.next().await;
                        if hold_counter > ON_HOLD_SLOW_THRESHOLD {
                            sleep(Duration::from_millis(
                                fec_sender.rtt().max(12) * (hold_counter - ON_HOLD_SLOW_THRESHOLD) as u64
                            ))
                            .await;
                        }

                        data
                    }
                    .fuse();
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

            let action = match (packet, feedback) {
                (Some((prefix, packet)), _) => fec_sender.send(prefix, packet)?,
                (_, Some(fb)) => {
                    use schema::devlog::bitbridge::fec_feedback::Feedback;
                    if let Feedback::Network(stats) = fb {
                        if let Some(peer_block_id) = stats.current_block_id {
                            if peer_block_id.abs_diff(fec_sender.block_id) < 2 {
                                hold_counter = 0;
                            }
                            else {
                                let peer_counter = stats.hold_counter.unwrap_or(0) as u8;
                                if peer_counter < hold_counter {
                                    hold_counter = hold_counter.saturating_sub(1);
                                }
                                else {
                                    hold_counter = hold_counter.min(ON_HOLD_STOP_THRESHOLD - peer_counter);
                                }
                            }
                        }
                    }

                    fec_sender.feedback(fb)
                }
                _ => break
            };

            match action {
                FecAction::Framed(frames) => {
                    let mut quad_channel = self.quad_unreliable_channel.lock().await;
                    for frame in frames {
                        let packet = frame.serialize();
                        buff_counter += packet.len();
                        let _ = quad_channel.send(self.peer.peer_id(), packet);
                    }
                }
                FecAction::Retransmit(frames) => {
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
                _ => {}
            }

            if buff_counter > MAX_BUFFER_SIZE {
                let tick = Instant::now();
                let mut quad_ch = self.quad_unreliable_channel.lock().await;
                let stats_before = quad_ch.bytes_sent().await;
                let timeout = Duration::from_millis(5 * fec_sender.rtt().max(100));

                quad_ch.wait_buffer_low(MIN_BUFFER_SIZE, timeout).await;
                self.buffer.wait_buffer_low(TRANSFER_RESOURCE_RELIABLE_CHANNEL_ID, MIN_BUFFER_SIZE, timeout).await;
                buff_counter = MIN_BUFFER_SIZE;

                hold_counter += 1;
                hold_counter = hold_counter.min(ON_HOLD_STOP_THRESHOLD);
                let hold_delimiter = TransferDelimiterShema::hold(hold_counter).as_bytes()?;
                let FecAction::Framed(frames) = fec_sender.send(0, hold_delimiter)? else {
                    return Err(anyhow!("Failed to build hold delimiter").into());
                };

                for frame in frames {
                    let _ = quad_ch.send(self.peer.peer_id(), frame.serialize());
                }

                let time = tick.elapsed().as_secs_f64().max(f64::MIN);
                let stats_after = quad_ch.bytes_sent().await;
                let total_sent = stats_after.saturating_sub(stats_before);
                let bw = (total_sent as f64 / time) as u64;
                let bw_kbps = bw / 1000;
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

        let timeout_token = CancellationToken::timeout(Duration::from_secs(6));

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
        let resource_order_id = resource.order_id;

        let transfer_id = TRANSFER_ID_COUNTER.fetch_add(1, Ordering::Relaxed);

        let request = DownloadResourceRequest {
            session_order_id,
            resource_order_id,
            transfer_id: transfer_id as u32
        };

        let (tx, mut rx) = mpsc::unbounded::<Packet>();

        let prefix = transfer_id;
        {
            let mut channels = self.prefix_channels.lock().await;
            channels.insert(prefix, tx);
        }

        let resource_token = self.transfers_context.get_or_create_resource_token(session_order_id, resource_order_id).await;

        log::info!("Requesting download for resource {:?}", request);
        self.msg_channel.notify(Request::DownloadResourceRequest(request)).await?;
        let resource_repo = self.resource_repo.clone();
        let prefix_channels = self.prefix_channels.clone();

        let start_delimiter = loop {
            match rx.next().with_cancel(&resource_token).await {
                Ok(Some(packet)) => {
                    if let Ok(delimiter) = TransferDelimiterShema::from_start_packet(&packet, session_order_id) {
                        break delimiter;
                    }
                }
                Ok(None) => {
                    log::warn!("Channel closed before receiving start delimiter");
                    return Err(WebRtcErrors::InvalidDelimiter("Channel closed before start delimiter".into()));
                }
                Err(_) => {
                    log::info!("Download cancelled while waiting for start delimiter");
                    return Err(WebRtcErrors::InvalidDelimiter("Download cancelled".into()));
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

        loop {
            match rx.next().with_cancel(&resource_token).await? {
                Some(packet) => {
                    if TransferDelimiterShema::from_end_packet(&packet, session_order_id).is_ok() {
                        log::info!("Received end delimiter for resource {}", resource_id);
                        progress.success();
                        core_request
                            .response(TransferOperationOutput::TransferResourceProgressUpdate(progress.clone()))
                            .await;
                        break;
                    }

                    if TransferDelimiterShema::from_hold_packet(&packet).is_ok() {
                        continue;
                    }

                    let bytes = Bytes::from(packet.to_vec());
                    let Some(written) = writer.d_write(bytes).await? else { continue };

                    progress.update_progress(written as u64);
                    core_request
                        .response_throttle(TransferOperationOutput::TransferResourceProgressUpdate(progress.clone()))
                        .await;
                }
                None => {
                    return Err(WebRtcErrors::InvalidDelimiter("Channel closed before end delimiter".into()));
                }
            }
        }

        prefix_channels.lock().await.remove(&prefix);

        log::info!("Completed download for resource {}", resource_id);

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
        match outbound_packet_sender.send((prefix, start_packet)).with_cancel(&resource_token).await {
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
            match cursor.c_next(None).with_cancel(&resource_token).await {
                Ok(Ok(Some((data, _raw_size)))) => {
                    if data.is_empty() {
                        break;
                    }

                    let packet = data.to_vec().into_boxed_slice();
                    match self.outbound_packet_sender.clone().send((prefix, packet)).with_cancel(&resource_token).await {
                        Ok(Ok(_)) => {}
                        Ok(Err(e)) => return Err(anyhow!("Failed to send data packet: {:?}", e).into()),
                        Err(_) => {
                            log::info!("Stream cancelled while sending data packet");
                            return Err(WebRtcErrors::InvalidDelimiter("Stream cancelled".into()));
                        }
                    }
                }
                Ok(Ok(None)) => {
                    break;
                }
                Ok(Err(e)) => {
                    log::error!("Error reading resource data: {:?}", e);
                    return Err(anyhow!("Failed to read resource: {:?}", e).into());
                }
                Err(_) => {
                    log::info!("Stream cancelled while reading resource data");
                    return Err(WebRtcErrors::InvalidDelimiter("Stream cancelled".into()));
                }
            }
        }

        let end_delimiter = TransferDelimiterShema::end(session_id, resource_id, compressed);
        let end_packet = end_delimiter.as_bytes()?;
        match outbound_packet_sender.send((prefix, end_packet)).with_cancel(&resource_token).await {
            Ok(Ok(_)) => {}
            Ok(Err(e)) => return Err(WebRtcErrors::InvalidDelimiter(format!("Failed to send end delimiter: {:?}", e))),
            Err(_) => {
                log::info!("Stream cancelled while sending end delimiter");
                return Err(WebRtcErrors::InvalidDelimiter("Stream cancelled".into()));
            }
        }

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

                let timeout_duration = next_check_time.take().map(|check_time| {
                    let now = Instant::now();
                    if check_time > now {
                        check_time.duration_since(now)
                    } else {
                        Duration::from_millis(0)
                    }
                });

                let packet_result = if let Some(timeout) = timeout_duration {
                    select! {
                        packet = data_rx.next().fuse() => packet,
                        _ = sleep(timeout).fuse() => None,
                    }
                } else {
                    data_rx.next().await
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

                        let channels = self.prefix_channels.lock().await;
                        if let Some(sender) = channels.get(&prefix) {
                            if let Err(e) = sender.unbounded_send(packet) {
                                log::warn!("Failed to send packet to prefix {} channel: {:?}", prefix, e);
                            }
                        } else {
                            log::warn!("No channel registered for prefix {}", prefix);
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
