use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use core_services::utils::yield_container::{YieldContainer, YieldError};
use futures::channel::mpsc;
use futures::channel::mpsc::unbounded;
use futures::stream::StreamExt;
use futures_timer::Delay;
use futures_util::lock::Mutex;
use futures_util::{select_biased, FutureExt, SinkExt};
use n0_future::time::Instant;
use once_cell::sync::OnceCell;
use schema::devlog::bitbridge::fec_feedback::Feedback;
use schema::devlog::bitbridge::peer_message_body::{Request, Response};
use schema::devlog::bitbridge::{FecFeedback, NetworkStats};
use shared::entities::peer::Peer as PeerEntity;
use shared::protocol::webrtc::fec::{FecAction, FecReceiver, Frame};
use shared::protocol::webrtc::message_channel::DirectMessageChannel;
use shared::protocol::webrtc::transfer::{TransferDelimiterShema, TransfersContext};
use shared::repository::local_resource::LocalResourceRepository;
use shared::repository::transfer_session::TransferSessionRepository;
use shared::shell::api::CoreRequest;

use crate::webrtc::ice::{IceAgent, IceError};
use crate::webrtc::signaling::{SignalingClient, SignalingError};
use crate::webrtc::web::channel_ids::*;
use crate::webrtc::web::{RtcConnectionWrapper, RtcDataChannelWrapper, WebError, WebRtcApi};

pub struct WebRtcClient {
    connection: Arc<RtcConnectionWrapper>,
    reliable_data_channel: Arc<RtcDataChannelWrapper>,
    unreliable_data_channel: Arc<RtcDataChannelWrapper>,
    unordered_data_channel: Arc<RtcDataChannelWrapper>,
    ordered_data_channel: Arc<RtcDataChannelWrapper>,
    transfers_context: TransfersContext,
    peer: OnceCell<PeerEntity>,
    core_request: OnceCell<CoreRequest>,
    resource_repo: Arc<dyn LocalResourceRepository>,
    transfer_session_repo: Arc<dyn TransferSessionRepository>,
    inbound_msg_receiver: YieldContainer<mpsc::UnboundedReceiver<Box<[u8]>>>,
    inbound_data_receiver: YieldContainer<mpsc::UnboundedReceiver<Box<[u8]>>>,
    signalling: SignalingClient,

    msg_channel: OnceCell<DirectMessageChannel>,
    unordered_msg_channel: OnceCell<DirectMessageChannel>,
    prefix_channels: Mutex<HashMap<u16, mpsc::Sender<Box<[u8]>>>>
}

fn spawn_outbound_sender(channel: Arc<RtcDataChannelWrapper>, mut rx: mpsc::Receiver<Box<[u8]>>) {
    wasm_bindgen_futures::spawn_local(async move {
        while let Some(data) = rx.next().await {
            let arr = js_sys::Uint8Array::from(&data[..]);
            let _ = channel.send_with_array_buffer_view(&arr);
        }
    });
}

impl WebRtcClient {
    pub async fn connect(
        signaling: SignalingClient,
        ice_agent: IceAgent,
        peer_id: &str,
        resource_repo: Arc<dyn LocalResourceRepository>,
        transfer_session_repo: Arc<dyn TransferSessionRepository>
    ) -> Result<Arc<Self>, WebRtcClientError> {
        log::info!("WebRtcClient connecting to peer {}", peer_id);

        let api = WebRtcApi::new("stun:stun.l.google.com:19302");
        let connection = api.create_peer_connection()?;

        let (msg_inbound_tx, msg_inbound_rx) = unbounded();
        let (data_inbound_tx, data_inbound_rx) = unbounded();
        let (ordered_out_tx, ordered_out_rx) = mpsc::channel(16);
        let (unordered_out_tx, unordered_out_rx) = mpsc::channel(16);

        let reliable_channel = api.create_reliable_channel(connection.clone(), RELIABLE_DATA_CHANNEL_ID)?;
        let unreliable_channel = api.create_unreliable_channel(connection.clone(), UNRELIABLE_DATA_CHANNEL_ID)?;
        let unordered_channel = api.create_unordered_channel(connection.clone(), UNORDERED_MSG_CHANNEL_ID)?;
        let ordered_channel = api.create_ordered_channel(connection.clone(), ORDERED_MSG_CHANNEL_ID)?;

        api.setup_channel_handlers(reliable_channel.clone(), data_inbound_tx.clone())?;
        api.setup_channel_handlers(unreliable_channel.clone(), data_inbound_tx)?;
        api.setup_channel_handlers(ordered_channel.clone(), msg_inbound_tx)?;

        api.create_offer_and_set_local(&connection).await?;

        ice_agent.wait_for_gathering_complete(&connection).await?;

        let local_sdp = connection
            .local_description()
            .ok_or_else(|| WebRtcClientError::Connection("No local description".to_string()))?
            .sdp();

        log::info!("ICE gathering complete, local SDP ready, sending to signaling {}", local_sdp);

        let answer_sdp = signaling.send_offer(peer_id, &local_sdp).await?;

        log::info!("Got answer from remote peer {answer_sdp:?}");
        api.set_remote_description(&connection, &answer_sdp).await?;

        let client = Arc::new(WebRtcClient {
            connection,
            reliable_data_channel: reliable_channel.clone(),
            unreliable_data_channel: unreliable_channel,
            unordered_data_channel: unordered_channel.clone(),
            ordered_data_channel: ordered_channel.clone(),
            transfers_context: TransfersContext::new(),
            peer: OnceCell::new(),
            core_request: OnceCell::new(),
            resource_repo,
            transfer_session_repo,
            inbound_msg_receiver: YieldContainer::new(msg_inbound_rx),
            inbound_data_receiver: YieldContainer::new(data_inbound_rx),
            signalling: signaling.clone(),
            msg_channel: OnceCell::new(),
            unordered_msg_channel: OnceCell::new(),
            prefix_channels: Mutex::new(HashMap::new())
        });

        let _ = client.msg_channel.set(DirectMessageChannel::new(ordered_out_tx));
        let _ = client.unordered_msg_channel.set(DirectMessageChannel::new(unordered_out_tx));

        spawn_outbound_sender(ordered_channel.clone(), ordered_out_rx);
        spawn_outbound_sender(unordered_channel.clone(), unordered_out_rx);

        api.wait_for_channel_open(ordered_channel).await?;

        log::info!("WebRtcClient connection established");
        Ok(client)
    }

    pub async fn run(self: Arc<Self>) -> Result<(), WebRtcClientError> {
        log::info!("WebRtcClient run loop starting");

        let msg_future = self.message_loop();
        let data_future = self.data_receiving_loop();

        futures::pin_mut!(msg_future, data_future);
        select_biased! {
            r = msg_future.fuse() => r?,
            r = data_future.fuse() => r?,
        }

        log::info!("WebRtcClient run loop terminated");
        Ok(())
    }

    pub fn peer_id(&self) -> Option<String> {
        self.peer.get().map(|p| p.id.clone())
    }

    pub fn peer_entity(&self) -> Option<PeerEntity> {
        self.peer.get().cloned()
    }

    pub fn set_peer(&self, peer: PeerEntity) -> Result<(), PeerEntity> {
        self.peer.set(peer)
    }

    pub async fn introduce(&self, current_user: &PeerEntity) -> Result<(), WebRtcClientError> {
        log::info!("Starting introduce handshake");

        let introduce_request = schema::devlog::bitbridge::IntroduceRequestMessage {
            mine: schema::devlog::bitbridge::PeerMessage {
                peer_id: current_user.id.clone(),
                name: current_user.name.clone(),
                avatar_url: current_user.avatar_url.clone(),
                device: current_user.device.clone().into(),
                email: current_user.email.clone()
            }
        };

        let msg_channel = self
            .msg_channel
            .get()
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
                    user_id: None,
                    signalling_id: None
                };
                self.set_peer(peer).map_err(|_| WebRtcClientError::Connection("Peer already set".to_string()))?;
                log::info!("Introduce handshake completed");
                Ok(())
            }
            _ => Err(WebRtcClientError::Message("Unexpected response type".to_string()))
        }
    }

    pub async fn from_introduce_request(
        self: Arc<Self>,
        request_id: String,
        msg: schema::devlog::bitbridge::IntroduceRequestMessage,
        current_user: &PeerEntity
    ) -> Result<(), WebRtcClientError> {
        log::info!("Creating WebRtcClient from introduce request");

        let peer = PeerEntity {
            id: msg.mine.peer_id.clone(),
            name: msg.mine.name.clone(),
            avatar_url: msg.mine.avatar_url.clone(),
            device: msg.mine.device.clone().into(),
            email: msg.mine.email.clone(),
            user_id: None,
            signalling_id: None
        };

        self.set_peer(peer).map_err(|_| WebRtcClientError::Connection("Peer already set".to_string()))?;

        let msg_channel = self
            .msg_channel
            .get()
            .ok_or_else(|| WebRtcClientError::Connection("No message channel".to_string()))?;

        let response = schema::devlog::bitbridge::IntroduceResponseMessage {
            peer: schema::devlog::bitbridge::PeerMessage {
                peer_id: current_user.id.clone(),
                name: current_user.name.clone(),
                avatar_url: current_user.avatar_url.clone(),
                device: current_user.device.clone().into(),
                email: current_user.email.clone()
            }
        };

        msg_channel
            .send_response(request_id, Response::IntroduceResponse(response))
            .await
            .map_err(|e| WebRtcClientError::Message(e.to_string()))?;
        log::info!("Sent introduce response to peer");
        Ok(())
    }

    pub async fn request_session_detail(
        &self,
        order_id: u64,
        password: Option<String>
    ) -> Result<schema::devlog::bitbridge::P2pTransferSessionMessage, WebRtcClientError> {
        use schema::devlog::bitbridge::view_session_detail_response::Result as ResponseResult;

        log::info!("Requesting session detail for order_id {}", order_id);

        let request = schema::devlog::bitbridge::ViewSessionDetailRequest { order_id, password };

        let msg_channel = self
            .msg_channel
            .get()
            .ok_or_else(|| WebRtcClientError::Connection("No message channel".to_string()))?;

        let response = msg_channel
            .send(Request::ViewSessionRequest(request), None)
            .await
            .map_err(|e| WebRtcClientError::Message(e.to_string()))?;

        match response {
            Response::ViewSessionResponse(resp) => match resp.result {
                Some(ResponseResult::Session(session)) => {
                    log::info!("Received session detail for order_id {}", order_id);
                    Ok(session)
                }
                Some(ResponseResult::Error(error_msg)) => {
                    log::error!("Session detail error: {:?}", error_msg);
                    Err(WebRtcClientError::Peer(format!("{:?}", error_msg)))
                }
                Some(ResponseResult::ResourceUpdated(_)) => Err(WebRtcClientError::Message("Unexpected resource updated".to_string())),
                None => Err(WebRtcClientError::Message("No result in response".to_string()))
            },
            _ => Err(WebRtcClientError::Message("Unexpected response type".to_string()))
        }
    }

    pub async fn request_resource_download(
        self: Arc<Self>,
        session_order_id: u64,
        resource_order_id: u64
    ) -> Result<(), WebRtcClientError> {
        use schema::devlog::bitbridge::DownloadResourceRequest;

        static TRANSFER_ID_COUNTER: std::sync::atomic::AtomicU16 = std::sync::atomic::AtomicU16::new(1);

        let transfer_id = TRANSFER_ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        let request = DownloadResourceRequest {
            session_order_id,
            resource_order_id,
            transfer_id: transfer_id as u32
        };

        let (tx, mut rx) = mpsc::channel::<Box<[u8]>>(64);
        self.prefix_channels.lock().await.insert(transfer_id, tx);

        log::info!(
            "Requesting download for resource {}, registered prefix channel: {}",
            resource_order_id,
            transfer_id
        );

        let msg_channel = self
            .msg_channel
            .get()
            .ok_or_else(|| WebRtcClientError::Connection("No message channel".to_string()))?;

        let _request_id = msg_channel
            .notify(Request::DownloadResourceRequest(request))
            .await
            .map_err(|e| WebRtcClientError::Message(e.to_string()))?;

        log::debug!("Sent download request with transfer_id: {}", transfer_id);

        while let Some(packet) = rx.next().await {
            log::debug!("Received packet of size {} on prefix channel", packet.len());
            self.process_data_packet(packet).await;
        }

        log::info!("Prefix channel {} closed", transfer_id);
        self.prefix_channels.lock().await.remove(&transfer_id);
        Ok(())
    }

    pub fn start_core_stream(&self, core_request: CoreRequest) {
        let _ = self.core_request.set(core_request);
    }

    pub fn core_request(&self) -> Option<&CoreRequest> {
        self.core_request.get()
    }

    pub async fn process_message_packet(&self, _request_id: String, msg: Request) {
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
                log::info!(
                    "Received resource notification for session order_id {}",
                    notification.session_order_id
                );
                let _ = notification;
            }
            _ => {
                log::debug!("Unhandled message request type");
            }
        }
    }

    pub async fn process_data_packet(&self, packet: Box<[u8]>) {
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

    async fn message_loop(&self) -> Result<(), WebRtcClientError> {
        log::info!("Starting message loop");

        let mut msg_rx = self.inbound_msg_receiver.retrieve().await?;

        let msg_channel = self
            .msg_channel
            .get()
            .ok_or_else(|| WebRtcClientError::Connection("No message channel".to_string()))?;

        while let Some(packet) = msg_rx.next().await {
            log::debug!("Received message packet of size {}", packet.len());

            if let Ok(Some(msg)) = msg_channel.receive_packet(packet).await {
                if let Some(request) = msg.request {
                    let request_id = msg.request_id;
                    self.process_message_packet(request_id, request).await;
                }
            }
        }

        log::info!("Message channel closed, terminating message loop");
        Ok(())
    }

    async fn data_receiving_loop(&self) -> Result<(), WebRtcClientError> {
        log::info!("Starting data receiving loop");

        let mut fec_receiver = FecReceiver::new();
        let mut data_rx = self.inbound_data_receiver.retrieve().await?;

        let unordered_msg_channel = self
            .unordered_msg_channel
            .get()
            .ok_or_else(|| WebRtcClientError::Connection("No unordered message channel".to_string()))?;

        let mut next_check_time: Option<Instant> = None;

        loop {
            let frames = {
                let mut frames = Vec::new();

                let check_time = next_check_time.take().unwrap_or_else(|| fec_receiver.calculate_next_check_time());
                let now = Instant::now();
                let timeout_duration = if check_time > now {
                    check_time.duration_since(now)
                } else {
                    Duration::from_millis(50)
                };

                let packet_result = {
                    select_biased! {
                        packet = data_rx.next().fuse() => packet,
                        _ = Delay::new(timeout_duration).fuse() => None,
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

            let action = if frames.is_empty() {
                fec_receiver.ping().map_err(|e| WebRtcClientError::Transfer(e.to_string()))?
            } else {
                fec_receiver.receive(frames).map_err(|e| WebRtcClientError::Transfer(e.to_string()))?
            };

            match action {
                FecAction::Constructed(packets_with_prefix, next_check) => {
                    next_check_time = Some(next_check);

                    let mut should_ack = false;
                    for (prefix, packet) in packets_with_prefix {
                        if let Ok(hold) = TransferDelimiterShema::from_hold_packet(&packet) {
                            let network_stats = NetworkStats {
                                current_block_id: Some(fec_receiver.current_block_id()),
                                rtt: Some(fec_receiver.rtt() as u32),
                                loss_rate: fec_receiver.calculate_loss_rate(),
                                hold_counter: hold.hold_counter().map(|it| it as u32)
                            };

                            let feedback = FecFeedback {
                                feedback: Some(Feedback::Network(network_stats))
                            };

                            let _ = unordered_msg_channel.notify(Request::FecFeedback(feedback)).await;
                            continue;
                        }

                        let sender = {
                            let channels = self.prefix_channels.lock().await;
                            channels.get(&prefix).cloned()
                        };

                        if let Some(mut sender) = sender {
                            if let Err(e) = sender.send(packet).await {
                                log::warn!("Prefix channel {} dropped: {:?}", prefix, e);
                                self.prefix_channels.lock().await.remove(&prefix);
                            } else {
                                should_ack = true;
                            }
                        }
                    }

                    if should_ack {
                        let feedback = FecFeedback {
                            feedback: Some(Feedback::Network(NetworkStats {
                                current_block_id: Some(fec_receiver.current_block_id()),
                                rtt: Some(fec_receiver.rtt() as u32),
                                loss_rate: fec_receiver.calculate_loss_rate(),
                                hold_counter: None
                            }))
                        };
                        let _ = unordered_msg_channel.notify(Request::FecFeedback(feedback)).await;
                    }
                }
                FecAction::Feedback(fb, next_check) => {
                    next_check_time = Some(next_check);
                    let _ = unordered_msg_channel.notify(Request::FecFeedback(fb)).await;
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

    #[error("Yield error: {0}")]
    Yield(#[from] YieldError)
}

impl From<SignalingError> for WebRtcClientError {
    fn from(err: SignalingError) -> Self {
        WebRtcClientError::Signaling(err.to_string())
    }
}

impl From<IceError> for WebRtcClientError {
    fn from(err: IceError) -> Self {
        WebRtcClientError::Connection(err.to_string())
    }
}

impl From<WebError> for WebRtcClientError {
    fn from(err: WebError) -> Self {
        WebRtcClientError::Connection(err.to_string())
    }
}

impl From<shared::errors::CoreError> for WebRtcClientError {
    fn from(err: shared::errors::CoreError) -> Self {
        WebRtcClientError::Transfer(err.to_string())
    }
}
