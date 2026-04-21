use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use core_services::utils::cancellation::FutureExtension;
use core_services::utils::yield_container::YieldContainer;
use futures::channel::mpsc as futures_mpsc;
use futures::SinkExt;
use futures_util::stream::StreamExt;
use prost::Message;
use schema::devlog::bitbridge::peer_message_body::{Request, Response};
use schema::devlog::bitbridge::view_session_detail_response::Result as SessionDetailResult;
use schema::devlog::bitbridge::*;
use schema::devlog::rpc_signalling::server::OfferMessage;
use tokio::sync::{mpsc, Notify, OnceCell};

use shared::app::operations::p2p::P2POperationOutput;
use shared::app::operations::CoreOperationOutput;
use shared::entities::local_resource::{LocalResource, LocalResourcePath, ResourceType};
use shared::entities::peer::Peer;
use shared::errors::CoreError;
use shared::protocol::webrtc::errors::WebRtcErrors;
use shared::protocol::webrtc::message_channel::DirectMessageChannel;
use shared::protocol::webrtc::packet::WebRtcPacket;
use shared::protocol::webrtc::transfer::{TransferDelimiterShema, TransfersContext};
use shared::repository::local_resource::LocalResourceRepository;
use shared::shell::api::CoreRequest;
use shared::utils::compression::is_compressible;

use crate::webrtc::pool::ConnectionPool;
use crate::webrtc::rtc::{RtcEvent, RtcHandle};
use crate::webrtc::signalling::SignallingSender;
use str0m::channel::ChannelId;

pub static CHUNK_SIZE: usize = 16 * 1024;
pub static MAX_BUFFER_SIZE: usize = 1024 * 1024 * 5;
pub static MIN_BUFFER_SIZE: usize = CHUNK_SIZE;
const RELIABLE_DATA_QUEUE_CAPACITY: usize = MAX_BUFFER_SIZE / CHUNK_SIZE + 1;
const OUTBOUND_RETRY_DELAY: Duration = Duration::from_millis(3);

pub type WebRtcClientError = WebRtcErrors;

pub struct WebRtcClient {
    msg_channel: OnceCell<DirectMessageChannel>,

    pool: OnceCell<Arc<ConnectionPool>>,
    pool_event_rx: YieldContainer<mpsc::Receiver<(usize, RtcEvent)>>,

    peer: OnceCell<Peer>,
    transfers_context: TransfersContext,
    core_request: OnceCell<CoreRequest>,
    resource_repo: Arc<dyn LocalResourceRepository>,
    session_id: OnceCell<u64>,

    outbound_packet_sender: OnceCell<futures_mpsc::Sender<(u16, u64, Vec<u8>, bool)>>,

    ordered_msg_rx: YieldContainer<futures_mpsc::Receiver<Vec<u8>>>,
    unordered_msg_rx: YieldContainer<futures_mpsc::Receiver<Vec<u8>>>,
    reliable_data_rx: YieldContainer<mpsc::Receiver<Vec<u8>>>,
    outbound_rx: YieldContainer<futures_mpsc::Receiver<(u16, u64, Vec<u8>, bool)>>,
    reliable_data_tx: YieldContainer<mpsc::Sender<Vec<u8>>>,
    bytes_sent_counter: Arc<AtomicUsize>,
    disconnect_requested: AtomicBool,
    disconnect_notify: Notify,
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
        signalling: SignallingSender,
        request_id: String,
        resource_repo: Arc<dyn LocalResourceRepository>,
        total_slots: usize,
    ) -> Result<Self, WebRtcClientError> {
        let Some(signalling_id) = me.signalling_id.clone() else {
            return Err(WebRtcClientError::Signalling("No signalling ID".to_string()));
        };

        let me_proto = schema::devlog::bitbridge::PeerMessage::from(me.clone());

        log::info!("[webrtc-client] Connecting via P2P (total_slots={total_slots})");

        let peer_from_offer = offer_message.peer.clone();

        let rtc_client = RtcHandle::connect(&signalling_id, offer_message, me_proto, signalling, &request_id).await?;

        let (ordered_msg_tx, ordered_msg_rx) = futures_mpsc::channel::<Vec<u8>>(64);
        let (_unordered_msg_tx, unordered_msg_rx) = futures_mpsc::channel::<Vec<u8>>(64);
        let (reliable_data_tx, reliable_data_rx) = mpsc::channel::<Vec<u8>>(RELIABLE_DATA_QUEUE_CAPACITY);
        let (outbound_tx, outbound_rx) = futures_mpsc::channel::<(u16, u64, Vec<u8>, bool)>(32);
        let (pool_event_tx, pool_event_rx) = mpsc::channel::<(usize, RtcEvent)>(32);

        let pool = ConnectionPool::new_with_primary(rtc_client, total_slots.max(1), pool_event_tx);

        let msg_channel = DirectMessageChannel::new(ordered_msg_tx);
        let peer: OnceCell<Peer> = OnceCell::new();

        let p = Peer::from(peer_from_offer);
        log::info!("[webrtc-client] Peer info received from signaled offer: {:?}", p.id);
        let _ = peer.set(p);

        let msg_channel_cell = OnceCell::new();
        let _ = msg_channel_cell.set(msg_channel);
        let outbound_packet_sender_cell = OnceCell::new();
        let _ = outbound_packet_sender_cell.set(outbound_tx);

        let pool_cell: OnceCell<Arc<ConnectionPool>> = OnceCell::new();
        let _ = pool_cell.set(pool);

        let client = Self {
            pool: pool_cell,
            pool_event_rx: YieldContainer::new(pool_event_rx),
            msg_channel: msg_channel_cell,
            peer,
            session_id: Default::default(),
            transfers_context: TransfersContext::new(),
            core_request: OnceCell::new(),
            resource_repo,
            outbound_packet_sender: outbound_packet_sender_cell,
            ordered_msg_rx: YieldContainer::new(ordered_msg_rx),
            unordered_msg_rx: YieldContainer::new(unordered_msg_rx),
            reliable_data_rx: YieldContainer::new(reliable_data_rx),
            outbound_rx: YieldContainer::new(outbound_rx),
            reliable_data_tx: YieldContainer::new(reliable_data_tx),
            bytes_sent_counter: Arc::new(AtomicUsize::new(0)),
            disconnect_requested: AtomicBool::new(false),
            disconnect_notify: Notify::new(),
        };

        log::info!("[webrtc-client] connection established, peer info exchanged via signaling.");

        Ok(client)
    }

    pub fn install_slot(
        self: &Arc<Self>,
        slot_idx: usize,
        offer_message: OfferMessage,
        me: Peer,
        signalling: SignallingSender,
        request_id: String,
        ice_config: Option<schema::devlog::rpc_signalling::server::IceConfig>,
    ) -> Result<(), WebRtcClientError> {
        let Some(pool) = self.pool.get().cloned() else {
            return Err(WebRtcClientError::Connection(
                "Connection pool not initialized; cannot install slot".into(),
            ));
        };

        let Some(signalling_id) = me.signalling_id.clone() else {
            return Err(WebRtcClientError::Signalling("No signalling ID for slot install".to_string()));
        };

        let me_proto = schema::devlog::bitbridge::PeerMessage::from(me);

        log::info!("[webrtc-client] Installing slot {slot_idx} for peer {:?}", self.peer_id());

        let connect_fut = async move {
            match ice_config {
                Some(cfg) => {
                    RtcHandle::connect_with_config(&signalling_id, offer_message, me_proto, signalling, &request_id, cfg).await
                }
                None => RtcHandle::connect(&signalling_id, offer_message, me_proto, signalling, &request_id).await,
            }
        };

        pool.spawn_lazy_slot(slot_idx, connect_fut);
        Ok(())
    }

    pub async fn run(self: Arc<Self>) -> Result<(), WebRtcClientError> {
        if let (Some(core_req), Some(p)) = (self.core_request.get(), self.peer.get()) {
            let _ = core_req.response(CoreOperationOutput::P2P(P2POperationOutput::PeerConnected(p.clone()))).await;
        }

        let pool = self
            .pool
            .get()
            .ok_or_else(|| WebRtcClientError::Connection("Connection pool not initialized".to_string()))?
            .clone();
        let cids = pool.channel_ids();

        let mut pool_event_rx_guard = self.pool_event_rx.retrieve().await?;
        let mut ordered_msg_rx_guard = self.ordered_msg_rx.retrieve().await?;
        let mut unordered_msg_rx_guard = self.unordered_msg_rx.retrieve().await?;
        let mut outbound_rx_guard = self.outbound_rx.retrieve().await?;
        let mut reliable_data_tx_guard = self.reliable_data_tx.retrieve().await?;

        let (msg_tx, msg_rx) = mpsc::unbounded_channel::<(String, Request)>();

        let mut reliable_data_rx_guard = self.reliable_data_rx.retrieve().await?;
        let reliable_data_tx = reliable_data_tx_guard.value.take().unwrap();
        let outbound_rx = outbound_rx_guard.value.take().unwrap();
        let reliable_data_rx = reliable_data_rx_guard.value.take().unwrap();
        let sending_handle = tokio::spawn(self.clone().sending_loop(reliable_data_tx, outbound_rx));

        let this_msg = self.clone();
        let msg_handle = tokio::spawn(async move {
            this_msg.msg_loop(msg_rx).await;
        });

        let mut pool_event_rx = pool_event_rx_guard.value.take().unwrap();
        let mut ordered_msg_rx = ordered_msg_rx_guard.value.take().unwrap();
        let mut unordered_msg_rx = unordered_msg_rx_guard.value.take().unwrap();
        let mut reliable_data_rx = reliable_data_rx;
        let mut pending_data: VecDeque<(Vec<u8>, ChannelId)> = VecDeque::new();

        let mut retry_timer = Box::pin(tokio::time::sleep(OUTBOUND_RETRY_DELAY));

        while pool.slot0_alive().await {
            if sending_handle.is_finished() || msg_handle.is_finished() {
                break;
            }

            if self.disconnect_requested.load(Ordering::SeqCst) {
                log::info!("[webrtc-client] Disconnect requested, stopping run loop");
                pool.shutdown_all().await;
                break;
            }

            let mut outbound_data = None;
            let mut flush_pending = false;

            tokio::select! {
                biased;

                _ = self.disconnect_notify.notified() => {
                    log::info!("[webrtc-client] Disconnect notification received");
                    pool.shutdown_all().await;
                    break;
                }

                () = &mut retry_timer, if !pending_data.is_empty() => {
                    flush_pending = true;
                }

                Some((slot_idx, rtc_event)) = pool_event_rx.recv() => {
                    flush_pending = matches!(
                        &rtc_event, RtcEvent::Str0mEvent(str0m::Event::ChannelBufferedAmountLow(cid))
                        if *cid == cids.reliable
                    );
                    if !self.handle_rtc_event(slot_idx, rtc_event, &cids, &msg_tx).await {
                        log::info!("[webrtc-client] Slot {slot_idx} reported terminal error");
                    }
                }

                Some(d) = ordered_msg_rx.next() => {
                    log::info!("Received ordered msg request");
                    outbound_data = Some((cids.ordered_msg, d));
                }
                Some(d) = unordered_msg_rx.next() => {
                    log::info!("Received unordered msg request");
                    outbound_data = Some((cids.unordered_msg, d));
                }
                Some(d) = reliable_data_rx.recv() => {
                    outbound_data = Some((cids.reliable, d));
                }
            }

            if let Some((cid, d)) = outbound_data {
                pending_data.push_back((d, cid));
                flush_pending = true;
            }

            if flush_pending {
                self.flush_pending_outbound(&mut pending_data, &pool, &cids).await;
                if !pending_data.is_empty() {
                    retry_timer.as_mut().reset(tokio::time::Instant::now() + OUTBOUND_RETRY_DELAY);
                }
            }
        }

        sending_handle.abort();
        msg_handle.abort();

        self.peer_disconnected().await;
        Ok(())
    }

    async fn flush_pending_outbound(
        &self,
        pending_data: &mut VecDeque<(Vec<u8>, ChannelId)>,
        pool: &Arc<ConnectionPool>,
        cids: &crate::webrtc::rtc::ChannelIds,
    ) {
        loop {
            let Some((data, channel_id)) = pending_data.pop_front() else {
                break;
            };

            let sent = if channel_id == cids.reliable {
                pool.try_send_reliable(&data).await
            } else {
                pool.try_send_control(&data, channel_id).await
            };

            if sent {
                if channel_id == cids.reliable {
                    self.bytes_sent_counter.fetch_add(data.len(), Ordering::Relaxed);
                }
                continue;
            }

            pending_data.push_front((data, channel_id));
            break;
        }
    }

    async fn handle_rtc_event(
        self: &Arc<Self>,
        _slot_idx: usize,
        event: RtcEvent,
        cids: &crate::webrtc::rtc::ChannelIds,
        msg_tx: &tokio::sync::mpsc::UnboundedSender<(String, Request)>,
    ) -> bool {
        match event {
            RtcEvent::Str0mEvent(event) => match event {
                str0m::Event::ChannelData(data) => {
                    if data.id == cids.ordered_msg {
                        if let Ok(msg) = PeerMessageBody::decode(&data.data[..]) {
                            if let Some(response) = msg.response {
                                self.msg_channel().notify_response(msg.request_id, response).await;
                            } else if let Some(request) = msg.request {
                                let _ = msg_tx.send((msg.request_id, request));
                            }
                        }
                    }
                }
                str0m::Event::IceConnectionStateChange(state) => {
                    log::info!("[webrtc-client] P2P ICE state: {:?}", state);
                }
                _ => {}
            },
            RtcEvent::Error(e) => {
                log::warn!("[webrtc-client] P2P RTC error: {e:?}");
                return false;
            }
        }

        true
    }

    async fn msg_loop(&self, mut msg_rx: mpsc::UnboundedReceiver<(String, Request)>) {
        while let Some((request_id, request)) = msg_rx.recv().await {
            self.process_message_packet(request_id, request).await;
        }

        log::info!("[webrtc-client] msg_loop ended");
    }

    pub fn start_core_stream(&self, core_request: CoreRequest) {
        let _ = self.core_request.set(core_request);
    }

    pub fn core_request(&self) -> Option<&CoreRequest> {
        self.core_request.get()
    }

    pub fn peer_id(&self) -> Option<String> {
        self.peer.get().map(|p| p.id.clone())
    }

    pub fn peer_entity(&self) -> Option<Peer> {
        self.peer.get().cloned()
    }

    pub fn session_id(&self) -> Option<u64> {
        self.session_id.get().copied()
    }

    pub fn disconnect(&self) {
        if self.disconnect_requested.swap(true, Ordering::SeqCst) {
            return;
        }

        log::info!("[webrtc-client] Disconnect requested for peer {:?}", self.peer_id());
        self.disconnect_notify.notify_one();
    }

    pub async fn stream_resource(&self, session_id: u64, transfer_id: u16, resource: LocalResource) -> Result<(), WebRtcClientError> {
        let resource_id = resource.order_id;
        let result = self.stream_resource_inner(session_id, transfer_id, resource).await;
        if let Err(ref e) = result {
            if matches!(e, WebRtcClientError::Canceled(_)) {
                log::info!("[webrtc-client] stream_resource canceled for resource {resource_id}");
            } else {
                log::warn!("[webrtc-client] stream_resource failed for resource {resource_id}: {e:?}");
                self.cancel_resource_transfer(session_id, resource_id).await;
            }
        }
        result
    }

    async fn stream_resource_inner(
        &self,
        session_id: u64,
        transfer_id: u16,
        resource: LocalResource,
    ) -> Result<(), WebRtcClientError> {
        let resource_id = resource.order_id;
        let prefix = transfer_id;
        let resource_token = self.transfers_context.get_or_create_resource_token(session_id, resource_id).await;

        let resource_name = match resource.r#type {
            ResourceType::Folder => format!("{}.zip", &resource.name),
            _ => resource.name.clone(),
        };

        log::info!("[webrtc-client] Streaming resource {resource_id} for transfer id {transfer_id}");
        let compressed = is_compressible(&resource_name);

        let start_delimiter = TransferDelimiterShema::start(session_id, resource_id, compressed);
        let start_packet = start_delimiter.as_bytes()?;
        let mut outbound_packet_sender = self
            .outbound_packet_sender
            .get()
            .ok_or(WebRtcClientError::Transfer("Outbound packet sender not initialized".into()))?
            .clone();

        outbound_packet_sender
            .send((prefix, 0, start_packet, true))
            .with_cancel(&resource_token)
            .await?
            .map_err(|e| WebRtcClientError::Transfer(format!("Failed to send start delimiter: {e:?}")))?;

        let mut cursor = self.resource_repo.read(resource.path.clone(), CHUNK_SIZE, compressed).await?;
        let mut current_offset: u64 = 0;

        loop {
            match cursor.c_next(None).await? {
                Some((data, raw_size)) => {
                    if data.is_empty() {
                        log::warn!("[webrtc-client] Cursor returned empty data");
                        break;
                    }
                    let packet = data.to_vec();
                    outbound_packet_sender
                        .send((prefix, current_offset, packet, false))
                        .with_cancel(&resource_token)
                        .await?
                        .map_err(|e| WebRtcClientError::Transfer(format!("Failed to send data packet: {e:?}")))?;

                    current_offset += raw_size as u64;
                }
                None => break,
            }
        }

        let end_delimiter = TransferDelimiterShema::end(session_id, resource_id, current_offset);
        let end_packet = end_delimiter.as_bytes()?;
        outbound_packet_sender
            .send((prefix, 0, end_packet, true))
            .with_cancel(&resource_token)
            .await?
            .map_err(|e| WebRtcClientError::Transfer(format!("Failed to send end delimiter: {e:?}")))?;

        log::info!("[webrtc-client] Completed streaming resource {resource_id} for session {session_id}");
        Ok(())
    }

    pub async fn send_resource_notification(&self, session_order_id: u64, resource: LocalResource) -> Result<(), WebRtcClientError> {
        let Some(mine_session_id) = self.session_id.get() else {
            log::error!("[webrtc-client] Session id not found");
            return Err(WebRtcClientError::Transfer("Session id not found".to_string()));
        };

        if *mine_session_id != session_order_id {
            return Ok(());
        }

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
            resource: Some(resource_proto),
        };

        self.msg_channel().notify(Request::ResourceNotification(notification)).await?;

        Ok(())
    }

    pub async fn send_session_detail_response(
        &self,
        request_id: String,
        session_message: Option<P2pTransferSessionMessage>,
        resources: Option<Vec<LocalResource>>,
        error: Option<CoreError>,
    ) -> Result<(), WebRtcClientError> {
        if let Some(error_msg) = error {
            log::error!("[webrtc-client] Session detail error: {error_msg:?}");
            let err_result = match error_msg {
                CoreError::PeerRequestError(e) => SessionDetailResult::Error(e.into()),
                _ => SessionDetailResult::Error(PeerErrorsMessage::InvalidRequest.into()),
            };
            self.msg_channel()
                .send_response(
                    request_id,
                    Response::ViewSessionResponse(ViewSessionDetailResponse { result: Some(err_result) }),
                )
                .await?;
            return Ok(());
        }

        let Some(proto_session) = session_message else {
            return Ok(());
        };

        log::info!(
            "[webrtc-client] Sending session detail: order_id={}, password_protected={}",
            proto_session.order_id,
            proto_session.password_protected
        );

        let response = ViewSessionDetailResponse {
            result: Some(SessionDetailResult::Session(proto_session.clone())),
        };
        self.msg_channel().send_response(request_id, Response::ViewSessionResponse(response)).await?;

        tokio::time::sleep(Duration::from_millis(100)).await;

        if let Some(resources) = resources {
            let session_order_id = proto_session.order_id;
            for resource in resources {
                log::debug!("[webrtc-client] Sending resource notification order_id={}", resource.order_id);
                self.send_resource_notification(session_order_id, resource).await?;
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        }

        Ok(())
    }

    pub async fn cancel_transfer(&self, session_id: u64) {
        self.transfers_context.cancel_transfer(session_id).await;
        let cancel_msg = P2pCancelSessionRequest {
            session_id,
            resource_id: None,
        };
        log::info!("[webrtc-client] Cancelling transfer session {session_id}");
        let _ = self.msg_channel().notify(Request::CancelRequest(cancel_msg)).await;
    }

    pub async fn cancel_resource_transfer(&self, session_id: u64, resource_id: u64) {
        self.transfers_context.cancel_resource(session_id, resource_id).await;
        let cancel_msg = P2pCancelSessionRequest {
            session_id,
            resource_id: Some(resource_id),
        };
        log::info!("[webrtc-client] Cancelling resource {resource_id} in session {session_id}");
        let _ = self.msg_channel().notify(Request::CancelRequest(cancel_msg)).await;
    }

    pub async fn peer_disconnected(&self) {
        log::info!("[webrtc-client] Peer disconnected, cancelling all transfers");
        self.transfers_context.cancel_all_transfers().await;
    }

    fn msg_channel(&self) -> &DirectMessageChannel {
        self.msg_channel.get().expect("Message channel not initialized")
    }

    pub async fn process_message_packet(&self, request_id: String, request: Request) {
        match request {
            Request::CancelRequest(req) => {
                log::info!("[webrtc-client] Received cancel request {:?}", req);
                if let Some(resource_id) = req.resource_id {
                    self.transfers_context.cancel_resource(req.session_id, resource_id).await;
                } else {
                    self.transfers_context.cancel_transfer(req.session_id).await;
                }
            }
            Request::FecFeedback(_) => {}
            request => {
                if let Some(core) = self.core_request.get() {
                    match request {
                        Request::ViewSessionRequest(msg) => {
                            let _ = self.session_id.set(msg.order_id);
                            core.response(CoreOperationOutput::P2P(P2POperationOutput::ReceivedViewSessionRequest {
                                peer_id: self.peer.get().unwrap().id.clone(),
                                request_id,
                                password: msg.password,
                                order_id: msg.order_id,
                            }))
                            .await;
                        }
                        Request::DownloadResourceRequest(req) => {
                            core.response(CoreOperationOutput::P2P(P2POperationOutput::ReceivedDownloadRequest {
                                peer_id: self.peer.get().unwrap().id.clone(),
                                session_order_id: req.session_order_id,
                                resource_order_id: req.resource_order_id,
                                transfer_id: req.transfer_id as u16,
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
                                        is_private: false,
                                    },
                                    thumbnail_path: None,
                                    r#type: ResourceTypeMessage::try_from(res.r#type).unwrap_or_default().into(),
                                    shelf_id: 0,
                                };
                                core.response(CoreOperationOutput::P2P(P2POperationOutput::ReceivedResourceNotification {
                                    peer_id: self.peer.get().unwrap().id.clone(),
                                    session_order_id: msg.session_order_id,
                                    resource,
                                }))
                                .await;
                            }

                            let _ = self.msg_channel().send_response(request_id, Response::VoidResponse(VoidResponseMessage {})).await;
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    async fn sending_loop(
        self: Arc<Self>,
        reliable_tx: mpsc::Sender<Vec<u8>>,
        mut outbound_rx: futures_mpsc::Receiver<(u16, u64, Vec<u8>, bool)>,
    ) {
        loop {
            let (prefix, offset, packet, _reliable) = match outbound_rx.next().await {
                Some(it) => it,
                None => break,
            };

            let data = WebRtcPacket::serialize(prefix, offset, &packet);

            if let Err(e) = reliable_tx.send(data).await {
                log::warn!("[webrtc-client] Failed to send to reliable_tx: {:?}", e);
            }
        }
    }
}
