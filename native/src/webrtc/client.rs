

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;



use core_services::utils::cancellation::{FutureExtension, TaskErrors};
use core_services::utils::yield_container::{YieldContainer, YieldError};
use futures::channel::mpsc as futures_mpsc;
use tokio::sync::mpsc;
use futures::SinkExt;
use futures_util::stream::StreamExt;
use prost::Message;
use schema::devlog::bitbridge::peer_message_body::{Request, Response};
use schema::devlog::bitbridge::view_session_detail_response::Result as SessionDetailResult;
use schema::devlog::bitbridge::*;
use schema::devlog::rpc_signalling::server::OfferMessage;
use thiserror::Error;
use tokio::sync::OnceCell;


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


use crate::webrtc::rtc::{RtcHandle, RtcEvent};
use crate::webrtc::signalling::SignallingSender;
use str0m::channel::ChannelId;


pub static CHUNK_SIZE: usize = 250 * 1024;
pub static MAX_BUFFER_SIZE: usize = 1024 * 1024 * 5;
pub static MIN_BUFFER_SIZE: usize = CHUNK_SIZE;

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
    Yield(#[from] YieldError),

    #[error("Task cancelled")]
    TaskCancelled(#[from] TaskErrors)
}

impl From<WebRtcErrors> for WebRtcClientError {
    fn from(err: WebRtcErrors) -> Self {
        WebRtcClientError::Shared(format!("{err}"))
    }
}

impl From<shared::repository::errors::PersistenceError> for WebRtcClientError {
    fn from(err: shared::repository::errors::PersistenceError) -> Self {
        WebRtcClientError::Repository(format!("{err}"))
    }
}

impl From<anyhow::Error> for WebRtcClientError {
    fn from(err: anyhow::Error) -> Self {
        WebRtcClientError::Shared(format!("{err}"))
    }
}

impl From<WebRtcClientError> for CoreError {
    fn from(err: WebRtcClientError) -> Self {
        CoreError::Network(format!("WebRtcClient {err:?}"))
    }
}

pub struct WebRtcClient {
    msg_channel: OnceCell<DirectMessageChannel>,

    p2p_rtc: YieldContainer<Option<RtcHandle>>,
    relay_rtc: YieldContainer<Option<RtcHandle>>,
    new_rtc_rx: YieldContainer<tokio::sync::mpsc::Receiver<(bool, RtcHandle)>>,

    peer: OnceCell<Peer>,
    me: Peer,
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
        session_id: String,
        signalling: SignallingSender,
        request_id: String,
        resource_repo: Arc<dyn LocalResourceRepository>
    ) -> Result<Self, WebRtcClientError> {
        let Some(signalling_id) = me.signalling_id.clone() else {
            return Err(WebRtcClientError::Shared("Peer not introduced".to_string()));
        };

        let signalling_id_p2p = signalling_id.clone();
        let signalling_p2p = signalling.clone();
        let request_id_p2p = request_id.clone();
        let offer_message_p2p = offer_message.clone();

        let signalling_id_relay = signalling_id.clone();
        let signalling_relay = signalling.clone();
        let session_id_relay = session_id.clone();

        let me_proto = schema::devlog::bitbridge::PeerMessage::from(me.clone());
        let me_proto_p2p = me_proto.clone();

        let p2p_fut: std::pin::Pin<Box<dyn std::future::Future<Output = Result<(bool, RtcHandle), WebRtcClientError>> + Send>> = Box::pin(async move {
            RtcHandle::connect(&signalling_id_p2p, offer_message_p2p, me_proto_p2p, signalling_p2p, &request_id_p2p)
                .await
                .map(|c| (true, c))
        });

        let relay_fut: std::pin::Pin<Box<dyn std::future::Future<Output = Result<(bool, RtcHandle), WebRtcClientError>> + Send>> = Box::pin(async move {
            RtcHandle::connect_relay(&signalling_id_relay, &session_id_relay, signalling_relay)
                .await
                .map(|c| (false, c))
        });

        let (first_res, mut remaining) = match futures::future::select_ok(vec![p2p_fut, relay_fut]).await {
            Ok((client, rem)) => (client, rem),
            Err(e) => return Err(WebRtcClientError::Signalling(format!("Both connection legs failed: {:?}", e))),
        };

        let first_conn = if first_res.0 { "truly P2P" } else { "Relay" };
        log::info!("[webrtc-client] First RTC connected (is_p2p: {}, type: {}), creating client", first_res.0, first_conn);

        let (new_rtc_tx, new_rtc_rx) = tokio::sync::mpsc::channel::<(bool, RtcHandle)>(1);
        if let Some(rem_fut) = remaining.pop() {
            tokio::spawn(async move {
                match rem_fut.await {
                    Ok(c) => {
                        let second_conn = if c.0 { "truly P2P" } else { "Relay" };
                        log::info!("[webrtc-client] Second RTC connected (is_p2p: {}, type: {})", c.0, second_conn);
                        let _ = new_rtc_tx.send(c).await;
                    }
                    Err(e) => log::error!("[webrtc-client] Remaining connection failed: {:?}", e),
                }
            });
        }

        let (ordered_msg_tx, ordered_msg_rx) = futures_mpsc::channel::<Vec<u8>>(64);
        let (_unordered_msg_tx, unordered_msg_rx) = futures_mpsc::channel::<Vec<u8>>(64);
        let (reliable_data_tx, reliable_data_rx) = mpsc::channel::<Vec<u8>>(MAX_BUFFER_SIZE / CHUNK_SIZE + 1);
        let (outbound_tx, outbound_rx) = futures_mpsc::channel::<(u16, u64, Vec<u8>, bool)>(32);

        let msg_channel = DirectMessageChannel::new(ordered_msg_tx);
        let first_rtc_client = first_res.1;
        let is_p2p = first_res.0;
        let peer: OnceCell<Peer> = OnceCell::new();

        if let Some(remote_peer_proto) = offer_message.peer {
            let p = Peer::from(remote_peer_proto);
            log::info!("[webrtc-client] Peer info received from signaled offer: {:?}", p.id);
            let _ = peer.set(p);
        } else {
            log::warn!("[webrtc-client] No peer info in signaled offer!");
        }

        let msg_channel_cell = OnceCell::new();
        let _ = msg_channel_cell.set(msg_channel);
        let outbound_packet_sender_cell = OnceCell::new();
        let _ = outbound_packet_sender_cell.set(outbound_tx);

        let mut p2p_rtc_opt = None;
        let mut relay_rtc_opt = None;
        if is_p2p {
            p2p_rtc_opt = Some(first_rtc_client);
        } else {
            relay_rtc_opt = Some(first_rtc_client);
        }

        let client = Self {
            me,
            p2p_rtc: YieldContainer::new(p2p_rtc_opt),
            relay_rtc: YieldContainer::new(relay_rtc_opt),
            new_rtc_rx: YieldContainer::new(new_rtc_rx),
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
        };

        log::info!("[webrtc-client] connection established, peer info exchanged via signaling.");

        Ok(client)
    }

    pub async fn run(self: Arc<Self>) -> Result<(), WebRtcClientError> {
        let mut p2p_guard = self.p2p_rtc.retrieve().await?;
        let mut relay_guard = self.relay_rtc.retrieve().await?;
        let mut new_rtc_rx_guard = self.new_rtc_rx.retrieve().await?;
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

        let mut new_rtc_rx = new_rtc_rx_guard.value.take().unwrap();
        let mut p2p_rtc = p2p_guard.value.take().unwrap();
        let mut relay_rtc = relay_guard.value.take().unwrap();

        let cids = if let Some(ref rtc) = p2p_rtc {
            *rtc.channel_ids()
        } else if let Some(ref rtc) = relay_rtc {
            *rtc.channel_ids()
        } else {
            return Err(WebRtcClientError::Shared("No connection available".to_string()));
        };

        let mut ordered_msg_rx = ordered_msg_rx_guard.value.take().unwrap();
        let mut unordered_msg_rx = unordered_msg_rx_guard.value.take().unwrap();
        let mut reliable_data_rx = reliable_data_rx;
        let mut pending_data: Option<(Vec<u8>, ChannelId)> = None;

        if let Some(ref mut rtc) = p2p_rtc {
            rtc.set_buffered_amount_low_threshold(cids.reliable, MIN_BUFFER_SIZE);
        }
        if let Some(ref mut rtc) = relay_rtc {
            rtc.set_buffered_amount_low_threshold(cids.reliable, MIN_BUFFER_SIZE);
        }

        let mut retry_timer = Box::pin(tokio::time::sleep(Duration::from_millis(3)));

        while p2p_rtc.as_ref().map_or(false, |r| r.is_alive()) || relay_rtc.as_ref().map_or(false, |r| r.is_alive()) || !new_rtc_rx.is_closed() {
            if sending_handle.is_finished() || msg_handle.is_finished() {
                break;
            }

            let mut outbound_data = None;

            tokio::select! {
                biased;

                // 1. New RTC connection arrivals from background racing
                Some(c) = new_rtc_rx.recv(), if !new_rtc_rx.is_closed() => {
                    c.1.set_buffered_amount_low_threshold(cids.reliable, MIN_BUFFER_SIZE);
                    if c.0 {
                        log::info!("[webrtc-client] truly P2P joined the run loop");
                        p2p_rtc = Some(c.1);
                    } else {
                        log::info!("[webrtc-client] Relay joined the run loop");
                        relay_rtc = Some(c.1);
                    }
                }

                // 2. Retry mechanism for pending outbound data blocked by backpressure
                () = &mut retry_timer, if pending_data.is_some() => {
                    let mut sent = false;
                    let (ref data, cid) = pending_data.as_ref().unwrap();
                    if let Some(ref rtc) = p2p_rtc {
                        if rtc.is_alive() && rtc.send(data, *cid) { sent = true; }
                    }
                    if !sent {
                        if let Some(ref rtc) = relay_rtc {
                            if rtc.is_alive() && rtc.send(data, *cid) { sent = true; }
                        }
                    }

                    if sent {
                        if pending_data.as_ref().unwrap().1 == cids.reliable {
                            self.bytes_sent_counter.fetch_add(data.len(), Ordering::Relaxed);
                        }
                        pending_data = None;
                    } else {
                        // Reset timer to retry again soon
                        retry_timer.as_mut().reset(tokio::time::Instant::now() + Duration::from_millis(3));
                    }
                }

                // 3. P2P events
                Some(rtc_event) = async {
                    match p2p_rtc.as_mut() {
                        Some(rtc) => rtc.poll_event().await,
                        None => std::future::pending().await,
                    }
                } => {
                    if !self.handle_rtc_event(rtc_event, &cids, &msg_tx, true).await {
                        p2p_rtc = None;
                    }
                }

                // 4. Relay events
                Some(rtc_event) = async {
                    match relay_rtc.as_mut() {
                        Some(rtc) => rtc.poll_event().await,
                        None => std::future::pending().await,
                    }
                } => {
                    if !self.handle_rtc_event(rtc_event, &cids, &msg_tx, false).await {
                        relay_rtc = None;
                    }
                }

                // 5. Outbound sending from queues (only processed if no pending_data is blocking)
                Some(d) = ordered_msg_rx.next(), if pending_data.is_none() => {
                    outbound_data = Some((cids.ordered_msg, d));
                }
                Some(d) = unordered_msg_rx.next(), if pending_data.is_none() => {
                    outbound_data = Some((cids.unordered_msg, d));
                }
                Some(d) = reliable_data_rx.recv(), if pending_data.is_none() => {
                    outbound_data = Some((cids.reliable, d));
                }
            }

            if let Some((cid, d)) = outbound_data {
                let mut sent = false;
                if let Some(ref rtc) = p2p_rtc {
                    if rtc.is_alive() && rtc.send(&d, cid) { sent = true; }
                }
                if !sent {
                    if let Some(ref rtc) = relay_rtc {
                        if rtc.is_alive() && rtc.send(&d, cid) { sent = true; }
                    }
                }

                if !sent {
                    pending_data = Some((d, cid));
                    retry_timer.as_mut().reset(tokio::time::Instant::now() + Duration::from_millis(3));
                } else if cid == cids.reliable {
                    self.bytes_sent_counter.fetch_add(d.len(), Ordering::Relaxed);
                }
            }
        }

        sending_handle.abort();
        msg_handle.abort();

        self.peer_disconnected().await;
        Ok(())
    }

    async fn handle_rtc_event(
        self: &Arc<Self>,
        event: RtcEvent,
        cids: &crate::webrtc::rtc::ChannelIds,
        msg_tx: &tokio::sync::mpsc::UnboundedSender<(String, Request)>,
        is_p2p: bool,
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
                str0m::Event::ChannelOpen(cid, _) => {
                    let label = if cid == cids.reliable {
                        "reliable"
                    } else if cid == cids.unordered_msg {
                        "unordered_msg"
                    } else if cid == cids.ordered_msg {
                        "ordered_msg"
                    } else {
                        "unknown"
                    };
                    log::info!(
                        "[webrtc-client] {} Data channel {} ({:?}) opened",
                        if is_p2p { "truly P2P" } else { "Relay" },
                        label,
                        cid
                    );
                }
                str0m::Event::IceConnectionStateChange(state) => {
                    log::info!("[webrtc-client] {} ICE state: {:?}", if is_p2p {"truly P2P"} else {"Relay"}, state);
                }
                _ => {}
            },
            RtcEvent::Error(e) => {
                log::warn!("[webrtc-client] {} RTC error: {e:?}", if is_p2p {"P2P"} else {"Relay"});
                return false;
            }
            RtcEvent::Closed => {
                log::info!("[webrtc-client] {} RTC closed", if is_p2p {"P2P"} else {"Relay"});
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

    pub async fn stream_resource(&self, session_id: u64, transfer_id: u16, resource: LocalResource) -> Result<(), WebRtcClientError> {
        let resource_id = resource.order_id;
        let result = self.stream_resource_inner(session_id, transfer_id, resource).await;
        if let Err(ref e) = result {
            log::warn!("[webrtc-client] stream_resource failed for resource {resource_id}: {e:?}");
            self.cancel_resource_transfer(session_id, resource_id).await;
        }
        result
    }

    async fn stream_resource_inner(
        &self,
        session_id: u64,
        transfer_id: u16,
        resource: LocalResource
    ) -> Result<(), WebRtcClientError> {
        let resource_id = resource.order_id;
        let prefix = transfer_id;
        let resource_token = self.transfers_context.get_or_create_resource_token(session_id, resource_id).await;

        let resource_name = match resource.r#type {
            ResourceType::Folder => format!("{}.zip", &resource.name),
            _ => resource.name.clone()
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
                None => break
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
            return Ok(())
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
            resource: Some(resource_proto)
        };

        self.msg_channel()
            .notify(Request::ResourceNotification(notification))
            .await
            .map(|_| ())
            .map_err(|e| WebRtcClientError::MessageChannel(format!("{e}")))?;

        Ok(())
    }

    pub async fn send_session_detail_response(
        &self,
        request_id: String,
        session_message: Option<P2pTransferSessionMessage>,
        resources: Option<Vec<LocalResource>>,
        error: Option<CoreError>
    ) -> Result<(), WebRtcClientError> {
        if let Some(error_msg) = error {
            log::error!("[webrtc-client] Session detail error: {error_msg:?}");
            let err_result = match error_msg {
                CoreError::PeerRequestError(e) => SessionDetailResult::Error(e.into()),
                _ => SessionDetailResult::Error(PeerErrorsMessage::InvalidRequest.into())
            };
            self.msg_channel()
                .send_response(
                    request_id,
                    Response::ViewSessionResponse(ViewSessionDetailResponse { result: Some(err_result) })
                )
                .await
                .map_err(|e| WebRtcClientError::MessageChannel(format!("{e}")))?;
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
            result: Some(SessionDetailResult::Session(proto_session.clone()))
        };
        self.msg_channel()
            .send_response(request_id, Response::ViewSessionResponse(response))
            .await
            .map_err(|e| WebRtcClientError::MessageChannel(format!("{e}")))?;

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
            resource_id: None
        };
        log::info!("[webrtc-client] Cancelling transfer session {session_id}");
        let _ = self.msg_channel().notify(Request::CancelRequest(cancel_msg)).await;
    }

    pub async fn cancel_resource_transfer(&self, session_id: u64, resource_id: u64) {
        self.transfers_context.cancel_resource(session_id, resource_id).await;
        let cancel_msg = P2pCancelSessionRequest {
            session_id,
            resource_id: Some(resource_id)
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
            Request::IntroduceRequest(msg) => {
                log::info!("[webrtc-client] Received introduce request from peer");
                let peer = Peer::from(msg.mine);
                log::info!("[webrtc-client] Remote peer: {:?}", peer.id);
                let _ = self.peer.set(peer);

                let response = IntroduceResponseMessage {
                    peer: PeerMessage::from(self.me.clone())
                };
                let _ = self.msg_channel().send_response(request_id, Response::IntroduceResponse(response)).await;
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
                                order_id: msg.order_id
                            }))
                            .await;
                        }
                        Request::DownloadResourceRequest(req) => {
                            core.response(CoreOperationOutput::P2P(P2POperationOutput::ReceivedDownloadRequest {
                                peer_id: self.peer.get().unwrap().id.clone(),
                                session_order_id: req.session_order_id,
                                resource_order_id: req.resource_order_id,
                                transfer_id: req.transfer_id as u16
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
        mut outbound_rx: futures_mpsc::Receiver<(u16, u64, Vec<u8>, bool)>
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
