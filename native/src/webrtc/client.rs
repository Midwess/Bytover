use std::ops::DerefMut;

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


use crate::webrtc::rtc::RtcClient;
use crate::webrtc::signalling::SignallingSender;
use str0m::channel::ChannelId;
use str0m::Event;

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

    rtc_client: YieldContainer<RtcClient>,

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

        let p2p_fut: std::pin::Pin<Box<dyn std::future::Future<Output = Result<RtcClient, WebRtcClientError>> + Send>> = Box::pin(RtcClient::connect(&signalling_id, offer_message.clone(), signalling.clone(), &request_id));
        let relay_fut: std::pin::Pin<Box<dyn std::future::Future<Output = Result<RtcClient, WebRtcClientError>> + Send>> = Box::pin(RtcClient::connect_relay(&signalling_id, &session_id, signalling));

        let mut rtc_client = match futures::future::select_ok(vec![p2p_fut, relay_fut]).await {
            Ok((client, _)) => client,
            Err(e) => return Err(WebRtcClientError::Signalling(format!("Both connection legs failed: {:?}", e))),
        };

        log::info!("[webrtc-client] RTC connected, creating client");

        let (ordered_msg_tx, mut ordered_msg_rx) = futures_mpsc::channel::<Vec<u8>>(64);
        let (_unordered_msg_tx, unordered_msg_rx) = futures_mpsc::channel::<Vec<u8>>(64);
        let (reliable_data_tx, reliable_data_rx) = mpsc::channel::<Vec<u8>>(MAX_BUFFER_SIZE / CHUNK_SIZE + 1);
        let (outbound_tx, outbound_rx) = futures_mpsc::channel::<(u16, u64, Vec<u8>, bool)>(32);

        let msg_channel = DirectMessageChannel::new(ordered_msg_tx);
        let cids = *rtc_client.channel_ids();
        let peer: OnceCell<Peer> = OnceCell::new();
        let mut introduced = false;
        // Responses queued during poll_event are buffered here and sent after the inner loop,
        // since RtcClient::send() and poll_event() both require &mut self.
        let mut pending_sends: Vec<(Vec<u8>, ChannelId)> = Vec::new();

        loop {
            while let Some(event) = rtc_client.poll_event().await? {
                if let Event::ChannelData(data) = event {
                    if data.id == cids.ordered_msg {
                        if let Ok(msg) = PeerMessageBody::decode(&data.data[..]) {
                            log::info!("Got msg {:?}", msg);
                            let request_id = msg.request_id;
                            if let Some(Response::IntroduceResponse(res)) = msg.response {
                                let p = Peer::from(res.peer);
                                log::info!("[webrtc-client] Peer introduced as {:?}", p.id);
                                let _ = peer.set(p);
                            } else if let Some(Request::IntroduceRequest(intro)) = msg.request {
                                let p = Peer::from(intro.mine);
                                log::info!("[webrtc-client] Remote peer: {:?}", p.id);
                                let _ = peer.set(p);
                                let mut bytes = vec![];
                                let response_body = PeerMessageBody {
                                    request_id,
                                    response: Some(Response::IntroduceResponse(IntroduceResponseMessage {
                                        peer: PeerMessage::from(me.clone())
                                    })),
                                    ..Default::default()
                                };
                                if response_body.encode(&mut bytes).is_ok() {
                                    pending_sends.push((bytes, cids.ordered_msg));
                                }
                            }
                        }
                    }
                }
            }

            for (data, channel_id) in pending_sends.drain(..) {
                let _ = rtc_client.send(&data, channel_id);
            }

            if peer.get().is_some() {
                break;
            }

            if !introduced {
                log::info!("[webrtc-client] Introducing self to peer");
                let _ = msg_channel
                    .notify(Request::IntroduceRequest(IntroduceRequestMessage {
                        mine: PeerMessage::from(me.clone())
                    }))
                    .await;
                introduced = true;
            }

            let timeout = rtc_client.timeout_duration();
            rtc_client.wait_for_input(timeout.min(Duration::from_millis(10))).await?;
            if let Ok(Some(data)) = ordered_msg_rx.try_next() {
                let _ = rtc_client.send(&data, cids.ordered_msg);
            }
        }

        let msg_channel_cell = OnceCell::new();
        let _ = msg_channel_cell.set(msg_channel);
        let outbound_packet_sender_cell = OnceCell::new();
        let _ = outbound_packet_sender_cell.set(outbound_tx);

        // rtc_client goes directly into YieldContainer here — it was never retrieved,
        // so run() can call retrieve() immediately without any async race.
        let client = Self {
            me,
            rtc_client: YieldContainer::new(rtc_client),
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

        Ok(client)
    }

    pub async fn run(self: Arc<Self>) -> Result<(), WebRtcClientError> {
        let mut rtc_container_guard = self.rtc_client.retrieve().await?;
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

        let rtc = rtc_container_guard.deref_mut();
        let cids = *rtc.channel_ids();

        let mut ordered_msg_rx = ordered_msg_rx_guard.value.take().unwrap();
        let mut unordered_msg_rx = unordered_msg_rx_guard.value.take().unwrap();
        let mut reliable_data_rx = reliable_data_rx;
        let mut pending_data: Option<(Vec<u8>, ChannelId)> = None;

        rtc.set_buffered_amount_low_threshold(cids.reliable, MIN_BUFFER_SIZE);

        while rtc.is_alive() {
            while let Some(event) = rtc.poll_event().await? {
                match event {
                    Event::ChannelData(data) => {
                        let id = data.id;
                        let data = data.data;
                        if id == cids.ordered_msg {
                            if let Ok(msg) = PeerMessageBody::decode(&data[..]) {
                                let request_id = msg.request_id;
                                if let Some(response) = msg.response {
                                    self.msg_channel().notify_response(request_id, response).await;
                                } else if let Some(request) = msg.request {
                                    let _ = msg_tx.send((request_id, request));
                                }
                            }
                        }
                    }
                    Event::IceConnectionStateChange(state) => {
                        log::info!("[webrtc-client] ICE state: {:?}", state);
                    }
                    _ => {}
                }
            }

            if sending_handle.is_finished() || msg_handle.is_finished() {
                break;
            }

            let timeout = rtc.timeout_duration();
            if let Some((ref data, cid)) = pending_data {
                if rtc.send(data, cid) {
                    pending_data = None;
                } else {
                    rtc.wait_for_input(timeout.max(Duration::from_millis(3))).await?;
                }
                continue;
            }

            let res = tokio::select! {
                Some(d) = ordered_msg_rx.next() => {
                    let d: Vec<u8> = d;
                    Ok::<_, WebRtcClientError>(Some((cids.ordered_msg, d)))
                },
                Some(d) = unordered_msg_rx.next() => {
                    let d: Vec<u8> = d;
                    Ok(Some((cids.unordered_msg, d)))
                },
                Some(d) = reliable_data_rx.recv() => {
                    let d: Vec<u8> = d;
                    Ok(Some((cids.reliable, d)))
                },
                res = rtc.wait_for_input(timeout.max(Duration::from_millis(3))) => {
                    res?;
                    Ok::<_, WebRtcClientError>(None)
                }
            }?;

            if let Some((cid, d)) = res {
                if !rtc.send(&d, cid) {
                    pending_data = Some((d, cid));
                    continue;
                }

                if cid == cids.reliable {
                    self.bytes_sent_counter.fetch_add(d.len(), Ordering::Relaxed);
                }
            }
        }

        sending_handle.abort();
        msg_handle.abort();

        self.peer_disconnected().await;
        Ok(())
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
