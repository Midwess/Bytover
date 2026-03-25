use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use prost::Message;
use str0m::change::SdpOffer;
use str0m::channel::ChannelId;
use str0m::net::{Protocol, Receive};
use str0m::{Candidate, Event, IceConnectionState, Input, Output, Rtc, RtcConfig};
use thiserror::Error;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::sync::{oneshot, Mutex, OnceCell, RwLock};
use devlog_sdk::distributed_id::gen_id;
use schema::devlog::bitbridge::peer_message_body::{Request, Response};
use schema::devlog::bitbridge::*;
use schema::devlog::rpc_signalling::server::OfferMessage;

use shared::app::operations::p2p::P2POperationOutput;
use shared::app::operations::transfer::TransferOperationOutput;
use shared::app::operations::CoreOperationOutput;
use shared::entities::local_resource::{LocalResource, LocalResourcePath, ResourceType};
use shared::entities::peer::Peer;
use shared::entities::transfer_session::TransferProgress;
use shared::errors::CoreError;
use shared::protocol::webrtc::transfer::{TransferDelimiterShema, TransfersContext};
use shared::repository::local_resource::LocalResourceRepository;
use shared::repository::transfer_session::TransferSessionRepository;
use shared::shell::api::CoreRequest;

use crate::webrtc::ice::IceAgent;
use crate::webrtc::signalling::SignalingClient;
use crate::webrtc::socket::{SyncUdpSocket, SyncUdpSocketError};

const TOTAL_CHANNELS: usize = 4;

const fn channel_id(raw: usize) -> ChannelId {
    unsafe { std::mem::transmute(raw) }
}

const MSG_CHANNEL_ID: ChannelId = channel_id(1);
const RELIABLE_DATA_CHANNEL_ID: ChannelId = channel_id(2);
const UNRELIABLE_DATA_CHANNEL_ID: ChannelId = channel_id(3);
const UNORDERED_MSG_CHANNEL_ID: ChannelId = channel_id(4);

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

    #[error("Socket error: {0}")]
    Socket(#[from] SyncUdpSocketError),

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
}

impl From<shared::protocol::webrtc::errors::WebRtcErrors> for WebRtcClientError {
    fn from(err: shared::protocol::webrtc::errors::WebRtcErrors) -> Self {
        WebRtcClientError::Shared(format!("{err}"))
    }
}

impl From<WebRtcClientError> for CoreError {
    fn from(err: WebRtcClientError) -> Self {
        CoreError::Network(format!("WebRtcClient {err:?}"))
    }
}

pub struct WebRtcClient {
    pub msg_tx: UnboundedSender<Vec<u8>>,
    pub reliable_data_tx: UnboundedSender<Vec<u8>>,
    pub unreliable_data_tx: UnboundedSender<Vec<u8>>,
    pub unordered_msg_tx: UnboundedSender<Vec<u8>>,

    peer: RwLock<Option<Peer>>,
    pending_responses: Mutex<HashMap<String, oneshot::Sender<Response>>>,
    transfers_context: TransfersContext,
    inbound_data_tx: UnboundedSender<Vec<u8>>,
    core_request: OnceCell<CoreRequest>,
    resource_repo: Arc<dyn LocalResourceRepository>,
    transfer_session_repo: Arc<dyn TransferSessionRepository>,
}

pub struct WebRtcTransport {
    rtc: Rtc,
    socket: SyncUdpSocket,
    signalling: SignalingClient,
    local_addr: SocketAddr,
    msg_rx: UnboundedReceiver<Vec<u8>>,
    reliable_data_rx: UnboundedReceiver<Vec<u8>>,
    unreliable_data_rx: UnboundedReceiver<Vec<u8>>,
    unordered_msg_rx: UnboundedReceiver<Vec<u8>>,
}

impl WebRtcClient {
    pub async fn connect(
        offer_message: OfferMessage,
        socket: SyncUdpSocket,
        signalling: SignalingClient,
        request_id: String,
        ice_agent: IceAgent,
        resource_repo: Arc<dyn LocalResourceRepository>,
        transfer_session_repo: Arc<dyn TransferSessionRepository>,
    ) -> Result<(Arc<Self>, WebRtcTransport), WebRtcClientError> {
        let mut rtc = RtcConfig::new().build(Instant::now());

        let local_addr = socket.local_addr()?;
        let host_candidate = Candidate::host(local_addr, "udp")
            .map_err(|e| WebRtcClientError::Signalling(format!("{e}")))?;
        rtc.add_local_candidate(host_candidate);
        log::info!("[webrtc-client] Added host candidate: {local_addr}");

        ice_agent.gather_candidates(&mut rtc, local_addr).await;

        let offer = SdpOffer::from_sdp_string(&offer_message.sdp)
            .map_err(|e| WebRtcClientError::SdpParse(format!("{e}")))?;

        let answer = rtc
            .sdp_api()
            .accept_offer(offer)
            .map_err(WebRtcClientError::Rtc)?;

        log::info!("[webrtc-client] SDP answer created with all local candidates");

        signalling
            .send_answer(answer.to_sdp_string(), &request_id)
            .await
            .map_err(|e| WebRtcClientError::Signalling(format!("{e}")))?;

        log::info!("[webrtc-client] Answer sent, waiting for connection and all channels");

        let (msg_tx, msg_rx) = unbounded_channel::<Vec<u8>>();
        let (reliable_data_tx, reliable_data_rx) = unbounded_channel::<Vec<u8>>();
        let (unreliable_data_tx, unreliable_data_rx) = unbounded_channel::<Vec<u8>>();
        let (unordered_msg_tx, unordered_msg_rx) = unbounded_channel::<Vec<u8>>();
        let (inbound_data_tx, _inbound_data_rx) = unbounded_channel::<Vec<u8>>();

        let mut channels_opened: usize = 0;
        let mut is_connected = false;

        let mut buf = vec![0u8; 2000];
        loop {
            let timeout = match rtc.poll_output()? {
                Output::Timeout(t) => t,
                Output::Transmit(t) => {
                    socket.send_to(&t.contents, t.destination).await?;
                    continue;
                }
                Output::Event(e) => {
                    match &e {
                        Event::Connected => {
                            log::info!("[webrtc-client] Connected");
                            is_connected = true;
                        }
                        Event::ChannelOpen(_, label) => {
                            channels_opened += 1;
                            log::info!(
                                "[webrtc-client] Channel {} opened (label: {})",
                                channels_opened,
                                label
                            );
                        }
                        Event::IceConnectionStateChange(state) => {
                            log::info!("[webrtc-client] ICE state: {:?}", state);
                            if matches!(state, IceConnectionState::Disconnected) {
                                return Err(WebRtcClientError::Signalling(
                                    "Peer disconnected during setup".into(),
                                ));
                            }
                        }
                        _ => {}
                    }

                    if is_connected && channels_opened >= TOTAL_CHANNELS {
                        log::info!("[webrtc-client] All channels open, ready");

                        let client = Arc::new(Self {
                            msg_tx,
                            reliable_data_tx,
                            unreliable_data_tx,
                            unordered_msg_tx,
                            peer: RwLock::new(None),
                            pending_responses: Mutex::new(HashMap::new()),
                            transfers_context: TransfersContext::new(),
                            inbound_data_tx,
                            core_request: OnceCell::new(),
                            resource_repo,
                            transfer_session_repo,
                        });

                        let transport = WebRtcTransport {
                            rtc,
                            socket,
                            signalling,
                            local_addr,
                            msg_rx,
                            reliable_data_rx,
                            unreliable_data_rx,
                            unordered_msg_rx,
                        };

                        return Ok((client, transport));
                    }
                    continue;
                }
            };

            let duration = timeout.saturating_duration_since(Instant::now());
            if duration.is_zero() {
                rtc.handle_input(Input::Timeout(Instant::now()))?;
                continue;
            }

            tokio::select! {
                result = socket.recv_any(&mut buf) => {
                    let (n, source) = result?;
                    let receive = Receive::new(Protocol::Udp, source, local_addr, &buf[..n])
                        .map_err(|e| WebRtcClientError::Signalling(format!("{e}")))?;
                    rtc.handle_input(Input::Receive(Instant::now(), receive))?;
                }
                _ = tokio::time::sleep(duration) => {
                    rtc.handle_input(Input::Timeout(Instant::now()))?;
                }
            }
        }
    }

    pub fn start_core_stream(&self, core_request: CoreRequest) {
        let _ = self.core_request.set(core_request);
    }

    pub fn core_request(&self) -> Option<&CoreRequest> {
        self.core_request.get()
    }

    pub async fn peer_id(&self) -> Option<String> {
        self.peer.read().await.as_ref().map(|p| p.id())
    }

    pub async fn introduce(&self, current_user: &Peer) -> Result<(), WebRtcClientError> {
        let introduce_request = IntroduceRequestMessage {
            mine: PeerMessage::from(current_user.clone()),
        };

        log::info!("[webrtc-client] Sending introduce request");
        let response = self
            .send_msg_request(Request::IntroduceRequest(introduce_request))
            .await?;

        match response {
            Response::IntroduceResponse(resp) => {
                let peer: Peer = resp.peer.into();
                log::info!("[webrtc-client] Received introduce response from {:?}", peer.id());
                *self.peer.write().await = Some(peer);
                Ok(())
            }
            _ => Err(WebRtcClientError::FailedToIntroducePeer),
        }
    }

    pub async fn from_introduce_request(
        &self,
        request_id: String,
        msg: IntroduceRequestMessage,
        current_user: &Peer,
    ) -> Result<(), WebRtcClientError> {
        log::info!("[webrtc-client] Received introduce request from {:?}", msg.mine.peer_id);

        let response = Response::IntroduceResponse(IntroduceResponseMessage {
            peer: PeerMessage::from(current_user.clone()),
        });

        self.send_msg_response(request_id, response).await?;

        let peer: Peer = msg.mine.into();
        *self.peer.write().await = Some(peer);
        Ok(())
    }

    pub async fn send_msg_request(&self, request: Request) -> Result<Response, WebRtcClientError> {
        let request_id = gen_id().await.to_string();
        let msg = PeerMessageBody {
            request: Some(request),
            request_id: request_id.clone(),
            ..Default::default()
        };

        let mut bytes = vec![];
        msg.encode(&mut bytes)?;

        let (tx, rx) = oneshot::channel();
        self.pending_responses.lock().await.insert(request_id.clone(), tx);

        self.msg_tx
            .send(bytes)
            .map_err(|e| WebRtcClientError::MessageChannel(format!("{e}")))?;

        match tokio::time::timeout(Duration::from_secs(60), rx).await {
            Ok(Ok(response)) => Ok(response),
            Ok(Err(_)) => {
                self.pending_responses.lock().await.remove(&request_id);
                Err(WebRtcClientError::MessageChannel("Response channel dropped".into()))
            }
            Err(_) => {
                self.pending_responses.lock().await.remove(&request_id);
                Err(WebRtcClientError::Timeout)
            }
        }
    }

    pub async fn notify_msg(&self, request: Request) -> Result<String, WebRtcClientError> {
        let request_id = gen_id().await.to_string();
        let msg = PeerMessageBody {
            request: Some(request),
            request_id: request_id.clone(),
            ..Default::default()
        };

        let mut bytes = vec![];
        msg.encode(&mut bytes)?;

        self.msg_tx
            .send(bytes)
            .map_err(|e| WebRtcClientError::MessageChannel(format!("{e}")))?;

        Ok(request_id)
    }

    pub async fn send_msg_response(
        &self,
        request_id: String,
        response: Response,
    ) -> Result<(), WebRtcClientError> {
        let msg = PeerMessageBody {
            request_id,
            response: Some(response),
            ..Default::default()
        };

        let mut bytes = vec![];
        msg.encode(&mut bytes)?;

        self.msg_tx
            .send(bytes)
            .map_err(|e| WebRtcClientError::MessageChannel(format!("{e}")))?;

        Ok(())
    }

    pub async fn notify_inbound_response(&self, request_id: String, response: Response) {
        let tx = self.pending_responses.lock().await.remove(&request_id);
        if let Some(tx) = tx {
            let _ = tx.send(response);
        }
    }

    pub async fn process_message_packet(&self, request_id: String, msg: Request) {
        match msg {
            Request::CancelRequest(request) => {
                log::info!("[webrtc-client] Received cancel request {:?}", request);
                if let Some(resource_id) = request.resource_id {
                    self.transfers_context.cancel_resource(request.session_id, resource_id).await;
                } else {
                    self.transfers_context.cancel_transfer(request.session_id).await;
                }
            }
            Request::ViewSessionRequest(req) => {
                log::info!("[webrtc-client] Received view session request for order_id {}", req.order_id);
                let peer_id = self.peer.read().await.as_ref().map(|p| p.id()).unwrap_or_default();
                let response = CoreOperationOutput::P2P(P2POperationOutput::ReceivedViewSessionRequest {
                    peer_id,
                    request_id,
                    order_id: req.order_id,
                    password: req.password,
                });

                if let Some(core_request) = self.core_request() {
                    core_request.response(response).await;
                }
            }
            Request::DownloadResourceRequest(req) => {
                let peer_id = self.peer.read().await.as_ref().map(|p| p.id()).unwrap_or_default();
                let response = CoreOperationOutput::P2P(P2POperationOutput::ReceivedDownloadRequest {
                    peer_id,
                    session_order_id: req.session_order_id,
                    resource_order_id: req.resource_order_id,
                    transfer_id: req.transfer_id as u16,
                });
                if let Some(core_request) = self.core_request() {
                    core_request.response(response).await;
                }
            }
            Request::ResourceNotification(notification) => {
                let session_order_id = notification.session_order_id;
                log::info!(
                    "[webrtc-client] Received resource notification for session {}",
                    session_order_id
                );
                if let Some(resource_proto) = notification.resource {
                    let mut resource = LocalResource {
                        order_id: resource_proto.order_id,
                        name: resource_proto.name,
                        size: resource_proto.size as u64,
                        path: LocalResourcePath::RelativePath {
                            path: format!(
                                "received/session_{}/resource_{}",
                                session_order_id, resource_proto.order_id
                            ),
                            is_private: false,
                        },
                        thumbnail_path: None,
                        r#type: (ResourceTypeMessage::try_from(resource_proto.r#type)
                            .unwrap_or_default())
                        .try_into()
                        .unwrap_or(ResourceType::File),
                        shelf_id: 0,
                    };

                    if let Some(thumbnail_bytes) = resource_proto.thumbnail_png {
                        match self.resource_repo.save_thumbnail(thumbnail_bytes, resource.order_id).await {
                            Ok(thumbnail_path) => {
                                resource.thumbnail_path = Some(thumbnail_path);
                            }
                            Err(e) => {
                                log::warn!("[webrtc-client] Failed to save thumbnail: {:?}", e);
                            }
                        }
                    }

                    if let Some(core_request) = self.core_request() {
                        let peer_id =
                            self.peer.read().await.as_ref().map(|p| p.id()).unwrap_or_default();
                        let response =
                            CoreOperationOutput::P2P(P2POperationOutput::ReceivedResourceNotification {
                                session_order_id,
                                resource,
                                peer_id,
                            });
                        core_request.response(response).await;
                    }
                }

                let _ = self
                    .send_msg_response(request_id, Response::VoidResponse(VoidResponseMessage {}))
                    .await;
            }
            Request::FecFeedback(_feedback) => {}
            _ => {}
        }
    }

    pub async fn process_data_packet(&self, _data: Vec<u8>) {
        // TODO: route to prefix channels when receiving_loop is implemented
    }

    pub async fn peer_disconnected(&self) {
        log::info!("[webrtc-client] Peer disconnected, cancelling all transfers");
        self.transfers_context.cancel_all_transfers().await;
        if let Some(core_request) = self.core_request() {
            core_request
                .response(CoreOperationOutput::P2P(P2POperationOutput::PeerDisconnected {}))
                .await;
        }
    }

    pub async fn cancel_transfer(&self, session_id: u64) {
        self.transfers_context.cancel_transfer(session_id).await;
        let peer_id = self.peer.read().await.as_ref().map(|p| p.id()).unwrap_or_default();
        log::info!("[webrtc-client] Cancelling transfer session {session_id} to peer {peer_id}");
        let _ = self
            .notify_msg(Request::CancelRequest(P2pCancelSessionRequest {
                session_id,
                resource_id: None,
            }))
            .await;
    }

    pub async fn cancel_resource_transfer(&self, session_id: u64, resource_id: u64) {
        self.transfers_context.cancel_resource(session_id, resource_id).await;
        log::info!(
            "[webrtc-client] Cancelling resource {resource_id} in session {session_id}"
        );
        let _ = self
            .notify_msg(Request::CancelRequest(P2pCancelSessionRequest {
                session_id,
                resource_id: Some(resource_id),
            }))
            .await;
    }

    pub async fn send_session_detail_response(
        &self,
        request_id: String,
        session_message: Option<P2pTransferSessionMessage>,
        resources: Option<Vec<LocalResource>>,
        error: Option<CoreError>,
    ) -> Result<(), WebRtcClientError> {
        use schema::devlog::bitbridge::view_session_detail_response::Result as ResponseResult;

        log::info!("[webrtc-client] Sending session detail response");

        if let Some(error_msg) = error {
            log::error!("[webrtc-client] Session detail error: {:?}", error_msg);
            let error_result = match error_msg {
                CoreError::PeerRequestError(e) => ResponseResult::Error(e.into()),
                _ => ResponseResult::Error(PeerErrorsMessage::InvalidRequest.into()),
            };
            self.send_msg_response(
                request_id,
                Response::ViewSessionResponse(ViewSessionDetailResponse {
                    result: Some(error_result),
                }),
            )
            .await?;
            return Ok(());
        }

        let Some(proto_session) = session_message else {
            return Ok(());
        };

        let response = ViewSessionDetailResponse {
            result: Some(ResponseResult::Session(proto_session.clone())),
        };

        self.send_msg_response(request_id, Response::ViewSessionResponse(response))
            .await?;

        tokio::time::sleep(Duration::from_millis(100)).await;

        if let Some(resources) = resources {
            let session_order_id = proto_session.order_id;
            for resource in resources {
                self.send_resource_notification(session_order_id, resource).await?;
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        }

        Ok(())
    }

    pub async fn send_resource_notification(
        &self,
        session_order_id: u64,
        resource: LocalResource,
    ) -> Result<(), WebRtcClientError> {
        let mut resource_proto = resource.to_proto();

        if let Some(thumbnail_path) = resource.thumbnail_path.as_ref() {
            if let Ok(mut cursor) = self.resource_repo.read(thumbnail_path.clone(), 64 * 1024, false).await {
                if let Ok(bytes) = cursor.read_all().await {
                    resource_proto.thumbnail_png = Some(bytes.to_vec());
                }
            }
        }

        let notification = ResourceNotificationRequest {
            session_order_id,
            resource: Some(resource_proto),
        };

        let _ = self.send_msg_request(Request::ResourceNotification(notification)).await?;
        Ok(())
    }

    pub async fn request_resource_download(
        &self,
        _core_request: CoreRequest,
        _session_order_id: u64,
        _resource: LocalResource,
        _progress: TransferProgress,
    ) -> Result<TransferProgress, WebRtcClientError> {
        todo!("request_resource_download requires receiving_loop")
    }

    pub async fn download_all_resources(
        &self,
        _core_request: CoreRequest,
        _session_order_id: u64,
        _session_resource: LocalResource,
        _resources: Vec<LocalResource>,
    ) -> Result<TransferProgress, WebRtcClientError> {
        todo!("download_all_resources requires receiving_loop")
    }

    pub async fn stream_resource(
        &self,
        _session_id: u64,
        _transfer_id: u16,
        _resource: LocalResource,
    ) -> Result<(), WebRtcClientError> {
        todo!("stream_resource requires sending_loop")
    }
}

impl WebRtcTransport {
    pub async fn run(mut self, client: Arc<WebRtcClient>) -> Result<(), WebRtcClientError> {
        let mut buf = vec![0u8; 2000];
        loop {
            if !self.rtc.is_alive() {
                client.peer_disconnected().await;
                return Ok(());
            }

            let timeout = loop {
                match self.rtc.poll_output()? {
                    Output::Timeout(t) => break t,
                    Output::Transmit(t) => {
                        self.socket.send_to(&t.contents, t.destination).await?;
                    }
                    Output::Event(e) => match e {
                        Event::IceConnectionStateChange(state) => {
                            log::info!("[webrtc-client] ICE state: {:?}", state);
                            if matches!(state, IceConnectionState::Disconnected) {
                                client.peer_disconnected().await;
                                self.rtc.disconnect();
                            }
                        }
                        Event::ChannelData(data) => {
                            if data.id == MSG_CHANNEL_ID || data.id == UNORDERED_MSG_CHANNEL_ID {
                                if let Ok(msg) = PeerMessageBody::decode(&data.data[..]) {
                                    let request_id = msg.request_id;
                                    if let Some(response) = msg.response {
                                        client.notify_inbound_response(request_id, response).await;
                                    } else if let Some(request) = msg.request {
                                        client.process_message_packet(request_id, request).await;
                                    }
                                }
                            } else if data.id == RELIABLE_DATA_CHANNEL_ID
                                || data.id == UNRELIABLE_DATA_CHANNEL_ID
                            {
                                client.process_data_packet(data.data).await;
                            }
                        }
                        _ => {}
                    },
                }
            };

            let duration = timeout
                .saturating_duration_since(Instant::now())
                .max(Duration::from_millis(1));

            tokio::select! {
                result = self.socket.recv_any(&mut buf) => {
                    let (n, source) = result?;
                    let Ok(receive) = Receive::new(Protocol::Udp, source, self.local_addr, &buf[..n]) else {
                        continue;
                    };
                    self.rtc.handle_input(Input::Receive(Instant::now(), receive))?;
                }
                _ = tokio::time::sleep(duration) => {
                    self.rtc.handle_input(Input::Timeout(Instant::now()))?;
                }
                Some(data) = self.msg_rx.recv() => {
                    if let Some(mut ch) = self.rtc.channel(MSG_CHANNEL_ID) {
                        ch.write(true, &data).ok();
                    }
                }
                Some(data) = self.reliable_data_rx.recv() => {
                    if let Some(mut ch) = self.rtc.channel(RELIABLE_DATA_CHANNEL_ID) {
                        ch.write(true, &data).ok();
                    }
                }
                Some(data) = self.unreliable_data_rx.recv() => {
                    if let Some(mut ch) = self.rtc.channel(UNRELIABLE_DATA_CHANNEL_ID) {
                        ch.write(true, &data).ok();
                    }
                }
                Some(data) = self.unordered_msg_rx.recv() => {
                    if let Some(mut ch) = self.rtc.channel(UNORDERED_MSG_CHANNEL_ID) {
                        ch.write(true, &data).ok();
                    }
                }
            }
        }
    }
}
