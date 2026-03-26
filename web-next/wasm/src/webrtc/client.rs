//! WebRTC Client for WASM (Receiving-Only)

use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

use futures::channel::mpsc;
use futures::channel::mpsc::unbounded;
use futures::stream::StreamExt;
use futures_util::lock::Mutex;
use futures_util::select_biased;
use futures_util::FutureExt;
use futures_timer::Delay;
use std::cell::RefCell;
use std::rc::Rc;
use js_sys::ArrayBuffer;
use js_sys::Uint8Array;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    Event, MessageEvent, RtcConfiguration, RtcDataChannel, RtcDataChannelInit,
    RtcDataChannelType, RtcIceConnectionState, RtcIceGatheringState,
    RtcIceServer, RtcPeerConnection, RtcSessionDescriptionInit, RtcSdpType,
};
use wasm_bindgen::JsValue;

use schema::devlog::bitbridge::peer_message_body::{Request, Response};
use shared::entities::peer::Peer as PeerEntity;
use shared::protocol::webrtc::message_channel::DirectMessageChannel;
use shared::protocol::webrtc::transfer::TransfersContext;
use shared::repository::local_resource::LocalResourceRepository;
use shared::repository::transfer_session::TransferSessionRepository;
use shared::shell::api::CoreRequest;

use crate::webrtc::signaling::{send_offer_proto, SignalingError};

pub struct RtcConnectionWrapper(pub(crate) RtcPeerConnection);

unsafe impl Send for RtcConnectionWrapper {}
unsafe impl Sync for RtcConnectionWrapper {}

impl Deref for RtcConnectionWrapper {
    type Target = RtcPeerConnection;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl RtcConnectionWrapper {
    pub fn new(conn: RtcPeerConnection) -> Arc<Self> {
        Arc::new(Self(conn))
    }
}

impl Drop for RtcConnectionWrapper {
    fn drop(&mut self) {
        log::info!("closing peer connection on drop");
        self.0.close();
    }
}

pub struct RtcDataChannelWrapper(pub(crate) RtcDataChannel);

unsafe impl Send for RtcDataChannelWrapper {}
unsafe impl Sync for RtcDataChannelWrapper {}

impl Deref for RtcDataChannelWrapper {
    type Target = RtcDataChannel;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for RtcDataChannelWrapper {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Drop for RtcDataChannelWrapper {
    fn drop(&mut self) {
        log::info!("closing data channel on drop");
        self.0.close();
    }
}

#[derive(Clone, Debug)]
pub struct ChannelConfig {
    pub ordered: bool,
    pub max_retransmits: Option<u16>,
    pub buffer_low_threshold: Option<usize>,
}

impl Default for ChannelConfig {
    fn default() -> Self {
        Self {
            ordered: true,
            max_retransmits: None,
            buffer_low_threshold: Some(16 * 1024),
        }
    }
}

pub mod channel_ids {
    pub const MSG_CHANNEL_ID: u16 = 0;
    pub const RELIABLE_CHANNEL_ID: u16 = 1;
    pub const UNRELIABLE_CHANNEL_ID: u16 = 2;
    pub const UNORDERED_CHANNEL_ID: u16 = 3;
}

use channel_ids::*;

#[derive(Default)]
struct IceCandidateTracker {
    host_candidates: usize,
    srflx_candidates: usize,
    relay_candidates: usize,
    prflx_candidates: usize,
}

impl IceCandidateTracker {
    fn add_candidate(&mut self, candidate: &str) {
        if candidate.contains("typ host") {
            self.host_candidates += 1;
        } else if candidate.contains("typ srflx") {
            self.srflx_candidates += 1;
        } else if candidate.contains("typ prflx") {
            self.prflx_candidates += 1;
        } else if candidate.contains("typ relay") {
            self.relay_candidates += 1;
        }
    }

    fn has_sufficient_candidates(&self) -> bool {
        (self.host_candidates > 0 || self.srflx_candidates > 0 || self.relay_candidates > 0)
            && self.prflx_candidates > 0
    }

    fn summary(&self) -> String {
        format!(
            "host={}, srflx={}, prflx={}, relay={}",
            self.host_candidates, self.srflx_candidates, self.prflx_candidates, self.relay_candidates
        )
    }
}

pub struct WebRtcClient {
    connection: Arc<RtcConnectionWrapper>,
    reliable_data_channel: Arc<RtcDataChannelWrapper>,
    unreliable_data_channel: Arc<RtcDataChannelWrapper>,
    unordered_data_channel: Arc<RtcDataChannelWrapper>,
    transfers_context: TransfersContext,
    peer: once_cell::sync::OnceCell<PeerEntity>,
    core_request: once_cell::sync::OnceCell<CoreRequest>,
    resource_repo: Arc<dyn LocalResourceRepository>,
    transfer_session_repo: Arc<dyn TransferSessionRepository>,
    inbound_data_stream_receiver: std::sync::Mutex<Option<mpsc::UnboundedReceiver<Box<[u8]>>>>,
    msg_channel: once_cell::sync::OnceCell<DirectMessageChannel>,
    prefix_channels: Mutex<HashMap<u16, mpsc::Sender<Box<[u8]>>>>,
}

impl WebRtcClient {
    pub async fn connect(
        signaling_url: &str,
        peer_id: &str,
        resource_repo: Arc<dyn LocalResourceRepository>,
        transfer_session_repo: Arc<dyn TransferSessionRepository>,
    ) -> Result<Arc<Self>, WebRtcClientError> {
        log::info!("WebRtcClient connecting to peer {}", peer_id);

        // Create peer connection with default STUN config
        let connection = create_rtc_peer_connection()?;

        // Create data channels before creating offer (needed for proper SDP)
        let (inbound_tx, inbound_rx) = unbounded();
        let (msg_sender, _msg_receiver) = mpsc::channel(16);
        let reliable_channel = create_reliable_channel(connection.clone(), MSG_CHANNEL_ID)?;
        let unreliable_channel = create_unreliable_channel(connection.clone(), UNRELIABLE_CHANNEL_ID)?;
        let unordered_channel = create_unordered_channel(connection.clone(), UNORDERED_CHANNEL_ID)?;

        // Setup handlers before creating offer
        setup_channel_event_handlers(
            reliable_channel.clone(),
            inbound_tx,
            ChannelConfig::default(),
        )?;

        // Create offer SDP
        let offer = JsFuture::from(connection.create_offer())
            .await
            .map_err(|e| WebRtcClientError::Connection(format!("create_offer failed: {:?}", e)))?;
        let sdp = js_sys::Reflect::get(&offer, &"sdp".into())
            .map_err(|e| WebRtcClientError::Connection(format!("Failed to get sdp: {:?}", e)))?
            .as_string()
            .ok_or_else(|| WebRtcClientError::Connection("SDP is not a string".to_string()))?;

        // Set local description
        let offer_desc = RtcSessionDescriptionInit::new(RtcSdpType::Offer);
        offer_desc.set_sdp(&sdp);
        JsFuture::from(connection.set_local_description(&offer_desc))
            .await
            .map_err(|e| WebRtcClientError::Connection(format!("set_local_description failed: {:?}", e)))?;

        // Wait for ICE gathering to complete
        wait_for_ice_gathering_complete(&connection).await?;

        // Get local SDP with ICE candidates embedded
        let local_sdp = connection
            .local_description()
            .ok_or_else(|| WebRtcClientError::Connection("No local description".to_string()))?
            .sdp();

        log::info!("ICE gathering complete, local SDP ready, sending to signaling");

        // Send offer to signaling server and receive answer
        let answer_sdp = send_offer_proto(signaling_url, peer_id, &local_sdp)
            .await?;

        // Set remote description (answer)
        let answer_desc = RtcSessionDescriptionInit::new(RtcSdpType::Answer);
        answer_desc.set_sdp(&answer_sdp);
        JsFuture::from(connection.set_remote_description(&answer_desc))
            .await
            .map_err(|e| WebRtcClientError::Connection(format!("set_remote_description failed: {:?}", e)))?;

        // Build client and wait for channel open
        let client = Arc::new(WebRtcClient {
            connection,
            reliable_data_channel: reliable_channel,
            unreliable_data_channel: unreliable_channel,
            unordered_data_channel: unordered_channel,
            transfers_context: TransfersContext::new(),
            peer: once_cell::sync::OnceCell::new(),
            core_request: once_cell::sync::OnceCell::new(),
            resource_repo,
            transfer_session_repo,
            inbound_data_stream_receiver: std::sync::Mutex::new(None),
            msg_channel: once_cell::sync::OnceCell::new(),
            prefix_channels: Mutex::new(HashMap::new()),
        });

        *client.inbound_data_stream_receiver.lock().unwrap() = Some(inbound_rx);
        let _ = client.msg_channel.set(DirectMessageChannel::new(msg_sender));

        wait_for_channel_open(client.reliable_data_channel.clone()).await?;

        log::info!("WebRtcClient connection established");
        Ok(client)
    }

    pub async fn run(self: Arc<Self>) -> Result<(), WebRtcClientError> {
        log::info!("WebRtcClient run loop starting");
        self.receiving_loop().await?;
        log::info!("WebRtcClient run loop terminated");
        Ok(())
    }

    pub fn peer_id(&self) -> Option<String> {
        self.peer.get().map(|p| p.id().to_string())
    }

    pub fn peer_entity(&self) -> Option<PeerEntity> {
        self.peer.get().cloned()
    }

    pub fn set_peer(&self, peer: PeerEntity) -> Result<(), PeerEntity> {
        self.peer.set(peer)
    }

    pub async fn introduce(
        &self,
        current_user: &PeerEntity,
    ) -> Result<(), WebRtcClientError> {
        log::info!("Starting introduce handshake");

        let introduce_request = schema::devlog::bitbridge::IntroduceRequestMessage {
            mine: schema::devlog::bitbridge::PeerMessage {
                peer_id: current_user.id().to_string(),
                name: current_user.name.clone(),
                avatar_url: current_user.avatar_url.clone(),
                device: current_user.device.clone().into(),
                email: current_user.email.clone(),
            },
        };

        let msg_channel = self.msg_channel.get()
            .ok_or_else(|| WebRtcClientError::Connection("No message channel".to_string()))?;

        let response = msg_channel
            .send(Request::IntroduceRequest(introduce_request), None)
            .await
            .map_err(|e| WebRtcClientError::Message(e.to_string()))?;

        match response {
            Response::IntroduceResponse(resp) => {
                let peer = PeerEntity {
                    id: resp.peer.peer_id.clone(),
                    name: resp.peer.name.clone(),
                    avatar_url: resp.peer.avatar_url.clone(),
                    device: resp.peer.device.clone().into(),
                    email: resp.peer.email.clone(),
                };
                self.set_peer(peer).map_err(|_| WebRtcClientError::Connection("Peer already set".to_string()))?;
                log::info!("Introduce handshake completed");
                Ok(())
            }
            _ => Err(WebRtcClientError::Message("Unexpected response type".to_string())),
        }
    }

    pub async fn from_introduce_request(
        self: Arc<Self>,
        request_id: String,
        msg: schema::devlog::bitbridge::IntroduceRequestMessage,
        current_user: &PeerEntity,
    ) -> Result<(), WebRtcClientError> {
        log::info!("Creating WebRtcClient from introduce request");

        let peer = PeerEntity {
            id: msg.mine.peer_id.clone(),
            name: msg.mine.name.clone(),
            avatar_url: msg.mine.avatar_url.clone(),
            device: msg.mine.device.clone().into(),
            email: msg.mine.email.clone(),
        };

        self.set_peer(peer).map_err(|_| WebRtcClientError::Connection("Peer already set".to_string()))?;

        let msg_channel = self.msg_channel.get()
            .ok_or_else(|| WebRtcClientError::Connection("No message channel".to_string()))?;

        let response = schema::devlog::bitbridge::IntroduceResponseMessage {
            peer: schema::devlog::bitbridge::PeerMessage {
                peer_id: current_user.id().to_string(),
                name: current_user.name.clone(),
                avatar_url: current_user.avatar_url.clone(),
                device: current_user.device.clone().into(),
                email: current_user.email.clone(),
            },
        };

        msg_channel.send_response(request_id, Response::IntroduceResponse(response)).await
            .map_err(|e| WebRtcClientError::Message(e.to_string()))?;
        log::info!("Sent introduce response to peer");
        Ok(())
    }

    pub async fn request_session_detail(
        &self,
        order_id: u64,
        password: Option<String>,
    ) -> Result<schema::devlog::bitbridge::P2pTransferSessionMessage, WebRtcClientError> {
        use schema::devlog::bitbridge::view_session_detail_response::Result as ResponseResult;

        log::info!("Requesting session detail for order_id {}", order_id);

        let request = schema::devlog::bitbridge::ViewSessionDetailRequest {
            order_id,
            password,
        };

        let msg_channel = self.msg_channel.get()
            .ok_or_else(|| WebRtcClientError::Connection("No message channel".to_string()))?;

        let response = msg_channel
            .send(Request::ViewSessionRequest(request), None)
            .await
            .map_err(|e| WebRtcClientError::Message(e.to_string()))?;

        match response {
            Response::ViewSessionResponse(resp) => {
                match resp.result {
                    Some(ResponseResult::Session(session)) => {
                        log::info!("Received session detail for order_id {}", order_id);
                        Ok(session)
                    }
                    Some(ResponseResult::Error(error_msg)) => {
                        log::error!("Session detail error: {:?}", error_msg);
                        Err(WebRtcClientError::Peer(format!("{:?}", error_msg)))
                    }
                    Some(ResponseResult::ResourceUpdated(_)) => {
                        Err(WebRtcClientError::Message("Unexpected resource updated".to_string()))
                    }
                    None => {
                        Err(WebRtcClientError::Message("No result in response".to_string()))
                    }
                }
            }
            _ => Err(WebRtcClientError::Message("Unexpected response type".to_string())),
        }
    }

    pub async fn request_resource_download(
        self: Arc<Self>,
        session_order_id: u64,
        resource_order_id: u64,
    ) -> Result<(), WebRtcClientError> {
        use schema::devlog::bitbridge::DownloadResourceRequest;

        static TRANSFER_ID_COUNTER: std::sync::atomic::AtomicU16 =
            std::sync::atomic::AtomicU16::new(1);

        let transfer_id = TRANSFER_ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        let request = DownloadResourceRequest {
            session_order_id,
            resource_order_id,
            transfer_id: transfer_id as u32,
        };

        let (tx, mut rx) = mpsc::channel::<Box<[u8]>>(64);
        self.prefix_channels.lock().await.insert(transfer_id, tx);

        log::info!(
            "Requesting download for resource {}, registered prefix channel: {}",
            resource_order_id,
            transfer_id
        );

        let msg_channel = self.msg_channel.get()
            .ok_or_else(|| WebRtcClientError::Connection("No message channel".to_string()))?;

        let _request_id = msg_channel.notify(Request::DownloadResourceRequest(request))
            .await
            .map_err(|e| WebRtcClientError::Message(e.to_string()))?;

        log::debug!("Sent download request with transfer_id: {}", transfer_id);

        loop {
            match rx.next().await {
                Some(packet) => {
                    log::debug!("Received packet of size {} on prefix channel", packet.len());
                    self.process_data_packet(packet).await;
                }
                None => {
                    log::info!("Prefix channel {} closed", transfer_id);
                    break;
                }
            }
        }

        self.prefix_channels.lock().await.remove(&transfer_id);
        Ok(())
    }

    pub fn start_core_stream(&self, core_request: CoreRequest) {
        let _ = self.core_request.set(core_request);
    }

    pub fn core_request(&self) -> Option<&CoreRequest> {
        self.core_request.get()
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
            Request::ResourceNotification(notification) => {
                log::info!("Received resource notification for session order_id {}", notification.session_order_id);
                let _ = notification;
            }
            _ => {
                log::debug!("Unhandled message request type");
            }
        }
    }

    pub async fn process_data_packet(&self, packet: Box<[u8]>) {
        // For now, just forward to prefix channel 0 if it exists
        let mut channels = self.prefix_channels.lock().await;
        if let Some(tx) = channels.get_mut(&0) {
            let _ = tx.try_send(packet);
        }
    }

    pub async fn cancel_transfer(&self, session_id: u64) {
        self.transfers_context.cancel_transfer(session_id).await;
    }

    pub async fn cancel_resource_transfer(&self, session_id: u64, resource_id: u64) {
        self.transfers_context.cancel_resource(session_id, resource_id).await;
    }

    pub async fn peer_disconnected(&self) {
        log::info!("Peer disconnected");
        self.transfers_context.cancel_all_transfers().await;
    }

    async fn receiving_loop(&self) -> Result<(), WebRtcClientError> {
        log::info!("Starting receiving loop");

        let mut inbound_rx = self
            .inbound_data_stream_receiver
            .lock()
            .unwrap()
            .take()
            .ok_or_else(|| WebRtcClientError::Connection("No inbound receiver".to_string()))?;

        let msg_channel = self.msg_channel.get()
            .ok_or_else(|| WebRtcClientError::Connection("No message channel".to_string()))?;

        futures::pin_mut!(inbound_rx);

        while let Some(packet) = inbound_rx.next().await {
            log::debug!("Received packet of size {}", packet.len());

            // Try to process as a response first via DirectMessageChannel
            if let Ok(Some(msg)) = msg_channel.receive_packet(packet).await {
                // If it's a request (not a response), handle it locally
                if let Some(request) = msg.request {
                    let request_id = msg.request_id;
                    self.process_message_packet(request_id, request).await;
                }
            }
        }

        log::info!("Inbound channel closed, terminating receiving loop");
        Ok(())
    }
}

fn create_rtc_peer_connection() -> Result<Arc<RtcConnectionWrapper>, WebRtcClientError> {
    let config = RtcConfiguration::new();

    // Use hardcoded Google STUN server
    let stun_server = RtcIceServer::new();
    stun_server.set_urls(&wasm_bindgen::JsValue::from_str("stun:stun.l.google.com:19302"));

    let ice_servers_array = js_sys::Array::new();
    ice_servers_array.push(&stun_server);
    config.set_ice_servers(&ice_servers_array);

    let connection = RtcPeerConnection::new_with_configuration(&config)
        .map_err(|e| WebRtcClientError::Connection(format!("Failed to create peer connection: {:?}", e)))?;

    Ok(RtcConnectionWrapper::new(connection))
}

fn create_reliable_channel(connection: Arc<RtcConnectionWrapper>, channel_id: u16) -> Result<Arc<RtcDataChannelWrapper>, WebRtcClientError> {
    let config = RtcDataChannelInit::new();
    config.set_ordered(true);
    config.set_negotiated(true);
    config.set_id(channel_id);

    let channel = connection
        .create_data_channel_with_data_channel_dict("reliable", &config);
    channel.set_binary_type(RtcDataChannelType::Arraybuffer);

    Ok(Arc::new(RtcDataChannelWrapper(channel)))
}

fn create_unreliable_channel(connection: Arc<RtcConnectionWrapper>, channel_id: u16) -> Result<Arc<RtcDataChannelWrapper>, WebRtcClientError> {
    let config = RtcDataChannelInit::new();
    config.set_ordered(false);
    config.set_max_retransmits(0);
    config.set_negotiated(true);
    config.set_id(channel_id);

    let channel = connection
        .create_data_channel_with_data_channel_dict("unreliable", &config);
    channel.set_binary_type(RtcDataChannelType::Arraybuffer);

    Ok(Arc::new(RtcDataChannelWrapper(channel)))
}

fn create_unordered_channel(connection: Arc<RtcConnectionWrapper>, channel_id: u16) -> Result<Arc<RtcDataChannelWrapper>, WebRtcClientError> {
    let config = RtcDataChannelInit::new();
    config.set_ordered(false);
    config.set_negotiated(true);
    config.set_id(channel_id);

    let channel = connection
        .create_data_channel_with_data_channel_dict("unordered", &config);
    channel.set_binary_type(RtcDataChannelType::Arraybuffer);

    Ok(Arc::new(RtcDataChannelWrapper(channel)))
}

fn setup_channel_event_handlers(
    channel: Arc<RtcDataChannelWrapper>,
    inbound_tx: mpsc::UnboundedSender<Box<[u8]>>,
    _config: ChannelConfig,
) -> Result<(), WebRtcClientError> {
    let onopen = {
        let channel = channel.clone();
        Closure::wrap(Box::new(move |_event: JsValue| {
            log::info!("Data channel {} opened", channel.id().unwrap_or(0));
        }) as Box<dyn FnMut(JsValue)>)
    };
    channel.clone().set_onopen(Some(onopen.as_ref().unchecked_ref()));
    onopen.forget();

    let onmessage = {
        let inbound_tx = inbound_tx.clone();
        let channel = channel.clone();
        Closure::wrap(Box::new(move |event: MessageEvent| {
            if let Ok(arraybuf) = event.data().dyn_into::<ArrayBuffer>() {
                let uarray = Uint8Array::new(&arraybuf);
                let body: Vec<u8> = uarray.to_vec();
                let packet: Box<[u8]> = body.into_boxed_slice();

                log::debug!("WASM received {} bytes on channel {}", packet.len(), channel.id().unwrap_or(0));

                if let Err(e) = inbound_tx.unbounded_send(packet) {
                    log::warn!("Failed to send inbound packet: {:?}", e);
                }
            }
        }) as Box<dyn FnMut(MessageEvent)>)
    };
    channel.clone().set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
    onmessage.forget();

    let onerror = {
        let channel = channel.clone();
        Closure::wrap(Box::new(move |event: JsValue| {
            log::error!("DataChannel {} error: {:?}", channel.id().unwrap_or(0), event);
        }) as Box<dyn FnMut(JsValue)>)
    };
    channel.clone().set_onerror(Some(onerror.as_ref().unchecked_ref()));
    onerror.forget();

    let onclose = {
        let channel = channel.clone();
        Closure::wrap(Box::new(move |_event: Event| {
            log::info!("Data channel {} closed", channel.id().unwrap_or(0));
        }) as Box<dyn FnMut(Event)>)
    };
    channel.set_onclose(Some(onclose.as_ref().unchecked_ref()));
    onclose.forget();

    Ok(())
}

async fn wait_for_ice_gathering_complete(conn: &RtcPeerConnection) -> Result<(), WebRtcClientError> {
    let timeout_ms = 30_000;
    let early_check_ms = 1_000;
    let cap_ms = 5_500;

    // Check if already complete
    if conn.ice_gathering_state() == RtcIceGatheringState::Complete {
        log::debug!("ICE gathering already complete");
        return Ok(());
    }

    let (complete_tx, mut complete_rx) = mpsc::channel::<bool>(1);

    // Track candidates for early exit
    let tracker = Rc::new(RefCell::new(IceCandidateTracker::default()));

    // Set up icegatheringstatechange handler
    let state_conn = conn.clone();
    let complete_tx_clone = complete_tx.clone();
    let onstatechange = Closure::wrap(Box::new(move || {
        if state_conn.ice_gathering_state() == RtcIceGatheringState::Complete {
            let _ = complete_tx_clone.clone().try_send(true);
        }
    }) as Box<dyn FnMut()>);
    conn.set_onicegatheringstatechange(Some(onstatechange.as_ref().unchecked_ref()));
    onstatechange.forget();

    // Set up onicecandidate handler (only for candidate tracking, not completion)
    let tracker_clone = tracker.clone();
    let oncandidate = Closure::wrap(Box::new(move |event: JsValue| {
        let candidate = js_sys::Reflect::get(&event, &"candidate".into());
        if let Ok(cand) = candidate {
            if !cand.is_null() {
                if let Ok(sdp) = js_sys::Reflect::get(&cand, &"candidate".into()) {
                    if let Some(sdp_str) = sdp.as_string() {
                        tracker_clone.borrow_mut().add_candidate(&sdp_str);
                    }
                }
            }
        }
    }) as Box<dyn FnMut(JsValue)>);
    conn.set_onicecandidate(Some(oncandidate.as_ref().unchecked_ref()));
    oncandidate.forget();

    let timeout = std::time::Duration::from_millis(timeout_ms.min(cap_ms) as u64);
    let early_check = std::time::Duration::from_millis(early_check_ms);

    select_biased! {
        _ = Delay::new(timeout).fuse() => {
            log::warn!("ICE gathering timed out after {}ms", timeout_ms);
        }
        _ = Delay::new(early_check).fuse() => {
            let ready = tracker.borrow().has_sufficient_candidates();
            if ready {
                log::debug!("ICE gathering early exit: sufficient candidates found");
                conn.set_onicegatheringstatechange(None);
                conn.set_onicecandidate(None);
                return Ok(());
            }
            let _ = complete_rx.next().await;
        }
        _ = complete_rx.next() => {}
    }

    // Clean up handlers
    conn.set_onicegatheringstatechange(None);
    conn.set_onicecandidate(None);

    log::debug!("ICE gathering done");
    Ok(())
}

async fn wait_for_channel_open(channel: Arc<RtcDataChannelWrapper>) -> Result<(), WebRtcClientError> {
    let (tx, mut rx) = mpsc::channel::<()>(1);

    let onopen = Closure::wrap(Box::new(move || {
        let _ = tx.clone().try_send(());
    }) as Box<dyn FnMut()>);

    channel.clone().set_onopen(Some(onopen.as_ref().unchecked_ref()));
    onopen.forget();

    rx.next().await;
    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum WebRtcClientError {
    #[error("Signaling error: {0}")]
    Signaling(String),

    #[error("Connection error: {0}")]
    Connection(String),

    #[error("Transfer error: {0}")]
    Transfer(String),

    #[error("Message error: {0}")]
    Message(String),

    #[error("Timeout")]
    Timeout,

    #[error("Peer error: {0}")]
    Peer(String),
}

impl From<SignalingError> for WebRtcClientError {
    fn from(err: SignalingError) -> Self {
        WebRtcClientError::Signaling(err.to_string())
    }
}

impl From<shared::errors::CoreError> for WebRtcClientError {
    fn from(err: shared::errors::CoreError) -> Self {
        WebRtcClientError::Transfer(err.to_string())
    }
}
