use crate::app::operations::p2p::P2POperationOutput;
use crate::entities::finding_scope::FindingScope;
use crate::entities::peer::Peer as PeerEntity;
use crate::entities::transfer_session::{TransferSession, TransferSessionStatus};
use crate::protocol::webrtc::errors::WebRtcErrors;
use crate::protocol::webrtc::message_channel::DirectMessageChannel;
use crate::protocol::webrtc::peer::WebRtcPeer;
use crate::protocol::webrtc::signalling::{SharedContext, WebSignallerBuilder};
use crate::repository::local_resource::LocalResourceRepository;
use crate::shell::api::CoreRequest;
use futures::select;
use futures_timer::Delay;
use futures_util::FutureExt;
use matchbox_protocol::PeerId;
use matchbox_socket::{ChannelConfig, WebRtcSocket};
use n0_future::task::spawn;
use prost::Message;
use schema::devlog::bitbridge::peer_message_body::Request;
use schema::devlog::bitbridge::PeerMessageBody;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;
use anyhow::anyhow;
use futures::executor::block_on;
use n0_future::time::sleep;
use crate::app::operations::CoreOperationOutput;
use crate::protocol::webrtc::fec::{CHUNK_SIZE, DATA_SHARDS_DEFAULT};

pub static MSG_CHANNEL_ID: usize = 0;
pub static TRANSFER_RESOURCE_RELIABLE_CHANNEL_ID: usize = 1;
pub static TRANSFER_RESOURCE_UNRELIABLE_CHANNEL_ID: usize = 2;
pub static TRANSFER_THUMBNAIL_CHANNEL_ID: usize = 3;
pub static TRANSFER_RESOURCE2_UNRELIABLE_CHANNEL_ID: usize = 4;

pub static MAX_BUFFER_SIZE: usize = 10 * CHUNK_SIZE * DATA_SHARDS_DEFAULT;
pub static MIN_BUFFER_SIZE: usize = 1;

pub struct WebRtc {
    addr: String,
    local_resource_repository: Arc<dyn LocalResourceRepository>,
    shared_context: SharedContext,
    is_running: AtomicBool
}

impl WebRtc {
    pub fn new(addr: String, local_resource_repository: Arc<dyn LocalResourceRepository>) -> Self {
        Self {
            addr,
            local_resource_repository,
            shared_context: SharedContext::new(),
            is_running: AtomicBool::new(false)
        }
    }

    pub fn is_running(&self) -> bool {
        self.is_running.load(std::sync::atomic::Ordering::SeqCst)
    }

    pub async fn stop(&self) {
        self.is_running.store(false, std::sync::atomic::Ordering::SeqCst);
        self.shared_context.remove_all().await;
        sleep(Duration::from_millis(500)).await;
        log::info!("Stopping WebRTC server");
    }

    pub async fn update_finding_scopes(&self, scopes: Vec<FindingScope>) {
        self.shared_context.update_finding_scopes(scopes).await;
    }

    pub async fn cancel_session(&self, peer_id: String, session_id: u64) -> Result<(), WebRtcErrors> {
        let peer_id = PeerId(peer_id.parse()?);
        if let Some(peer) = self.shared_context.get_peer(&peer_id).await.and_then(|peer| peer.upgrade()) {
            peer.cancel_transfer(session_id).await;

            return Ok(())
        };

        Err(WebRtcErrors::ConnectionNotFound(peer_id))
    }

    pub async fn answer_session(
        &self,
        core_request: CoreRequest,
        peer_id: String,
        session: Option<TransferSession>,
        session_id: u64
    ) -> Result<TransferSessionStatus, WebRtcErrors> {
        let peer_id = PeerId(peer_id.parse()?);

        if let Some(peer) = self.shared_context.get_peer(&peer_id).await.and_then(|peer| peer.upgrade()) {
            let result = peer.answer_transfer(core_request, session_id, session).await;

            return result;
        };

        Err(WebRtcErrors::ConnectionNotFound(peer_id))
    }

    pub async fn send_session(
        &self,
        core_request: CoreRequest,
        session: TransferSession
    ) -> Result<TransferSessionStatus, WebRtcErrors> {
        let Some(peer_id) = session.peer().map(|it| it.peer_id()) else {
            return Err(anyhow!("This session is not a peer session").into())
        };

        if let Some(peer) = self.shared_context.get_peer(&peer_id).await.and_then(|peer| peer.upgrade()) {
            let result = peer.transfer_session(core_request, session).await;

            return result;
        };

        Err(WebRtcErrors::ConnectionNotFound(peer_id))
    }

    pub async fn start_peer_core_stream(&self, peer_id: String, core_request: CoreRequest) -> Result<(), WebRtcErrors> {
        let peer_id = PeerId(peer_id.parse()?);
        if let Some(peer) = self.shared_context.get_peer(&peer_id).await.and_then(|peer| peer.upgrade()) {
            peer.start_core_stream(core_request);
            return Ok(());
        } else {
            log::info!("Peer not found");
        }

        Err(WebRtcErrors::ConnectionNotFound(peer_id))
    }

    pub async fn start(&self, core_request: CoreRequest, current_user: PeerEntity) -> Result<(), WebRtcErrors> {
        if self.is_running() {
            log::info!("The webrtc server is already running");
            core_request.response(P2POperationOutput::AlreadyRunning).await;
            return Ok(())
        }

        self.is_running.store(true, std::sync::atomic::Ordering::SeqCst);
        self.shared_context.set_current_id(current_user.peer_id()).await;
        log::info!("Starting WebRTC server with my peer = {current_user:?}");
        let signaller_builder = Arc::new(WebSignallerBuilder::new(self.shared_context.clone()));
        let (mut socket, loop_fut) = WebRtcSocket::builder(self.addr.clone())
            .signaller_builder(signaller_builder.clone())
            .add_reliable_channel(Some(MIN_BUFFER_SIZE)) // Msg
            .add_reliable_channel(Some(MIN_BUFFER_SIZE)) // Resource reliable, for retransmissions and delimiter
            .add_unreliable_channel(Some(MIN_BUFFER_SIZE)) // Resource unreliable, for retransmissions
            .add_reliable_channel(Some(MIN_BUFFER_SIZE)) // Thumbnail
            .add_unreliable_channel(Some(MIN_BUFFER_SIZE)) // Resource2 unreliable, for retransmissions
            .signaling_keep_alive_interval(Some(Duration::from_millis(3500)))
            .reconnect_attempts(Some(u16::MAX))
            .handshake_timeout(Duration::from_secs(10))
            .build();

        let loop_fut = loop_fut.fuse();
        futures::pin_mut!(loop_fut);
        let timeout = Delay::new(Duration::from_millis(8));
        futures::pin_mut!(timeout);

        let outbound_msg_sender = socket.channel(MSG_CHANNEL_ID).sender_clone();
        let outbound_reliable_data_sender = socket.channel(TRANSFER_RESOURCE_RELIABLE_CHANNEL_ID).sender_clone();
        let outbound_unreliable_data_sender = socket.channel(TRANSFER_RESOURCE_UNRELIABLE_CHANNEL_ID).sender_clone();
        let outbound_thumbnail_sender = socket.channel(TRANSFER_THUMBNAIL_CHANNEL_ID).sender_clone();
        let outbound_unreliable2_data_sender = socket.channel(TRANSFER_RESOURCE2_UNRELIABLE_CHANNEL_ID).sender_clone();

        let mut handles = vec![];

        let result = loop {
            if !self.is_running.load(std::sync::atomic::Ordering::Relaxed) {
                log::info!("The webrtc server is stopped, will cleanup");
                break Ok(());
            }

            for (peer_id, state) in socket.try_update_peers()? {
                if state == matchbox_socket::PeerState::Connected {
                    log::info!("Peer {peer_id} connected");
                    if self.shared_context.is_peer_connected(&peer_id).await {
                        continue;
                    }

                    if peer_id > current_user.peer_id() {
                        let Some(buffer) = socket.get_peer_buffer_info(peer_id).cloned() else {
                            log::error!("Buffer not found for peer {peer_id}");
                            continue;
                        };

                        let direct_message_channel = DirectMessageChannel::new(peer_id, outbound_msg_sender.clone());
                        let current_user = current_user.clone();
                        let outbound_reliable_data_sender = outbound_reliable_data_sender.clone();
                        let outbound_unreliable_data_sender = outbound_unreliable_data_sender.clone();
                        let outbound_unreliable2_data_sender = outbound_unreliable2_data_sender.clone();
                        let outbound_thumbnail_sender = outbound_thumbnail_sender.clone();
                        let local_resource_repository = self.local_resource_repository.clone();
                        let context = self.shared_context.clone();
                        let core_request = core_request.clone();
                        self.shared_context.add_peer_msg_channel(&peer_id, &direct_message_channel).await;
                        handles.push(spawn(async move {
                            let peer = match WebRtcPeer::new(
                                current_user.clone(),
                                direct_message_channel,
                                outbound_reliable_data_sender,
                                outbound_unreliable_data_sender,
                                outbound_unreliable2_data_sender,
                                outbound_thumbnail_sender,
                                buffer,
                                local_resource_repository
                            )
                            .await
                            {
                                Ok(peer) => peer,
                                Err(err) => {
                                    log::error!("Failed to connect to peer {err:?}");
                                    return;
                                }
                            };

                            let peer_entity = peer.peer.clone();
                            context.add_peer(peer).await;
                            let _ = core_request.response(P2POperationOutput::PeerConnected(peer_entity)).await;
                        }));
                    }
                }
                else if state == matchbox_socket::PeerState::Disconnected {
                    log::info!("Peer {peer_id} disconnected");
                    let context = self.shared_context.clone();
                    spawn(async move {
                        context.remove_peer(&peer_id).await;
                    });
                }
            }

            for (peer_id, msg) in socket.channel_mut(MSG_CHANNEL_ID).receive() {
                let Ok(msg) = PeerMessageBody::decode(&msg[..]) else {
                    continue;
                };

                if let Some(Request::IntroduceRequest(request)) = msg.request {
                    let Some(buffer) = socket.get_peer_buffer_info(peer_id).cloned() else {
                        log::error!("Buffer not found for peer {peer_id}");
                        continue;
                    };
                    let direct_message_channel = DirectMessageChannel::new(peer_id, outbound_msg_sender.clone());
                    let current_user = current_user.clone();
                    let peer_id = peer_id;
                    let outbound_reliable_data_sender = outbound_reliable_data_sender.clone();
                    let outbound_unreliable_data_sender = outbound_unreliable_data_sender.clone();
                    let outbound_unreliable2_data_sender = outbound_unreliable2_data_sender.clone();
                    let outbound_thumbnail_sender = outbound_thumbnail_sender.clone();
                    let local_resource_repository = self.local_resource_repository.clone();
                    let request_id = msg.request_id.clone();
                    let core_request = core_request.clone();
                    let context = self.shared_context.clone();
                    context.add_peer_msg_channel(&peer_id, &direct_message_channel).await;
                    handles.push(spawn(async move {
                        let peer = match WebRtcPeer::from_introduce_request(
                            current_user,
                            request_id,
                            request,
                            direct_message_channel,
                            outbound_reliable_data_sender,
                            outbound_unreliable_data_sender,
                            outbound_unreliable2_data_sender,
                            outbound_thumbnail_sender,
                            buffer,
                            local_resource_repository
                        )
                        .await
                        {
                            Ok(peer) => peer,
                            Err(err) => {
                                log::error!("Failed to connect to peer {err:?}");
                                return;
                            }
                        };

                        let peer_entity = peer.peer.clone();
                        context.add_peer(peer).await;
                        let _ = core_request.response(P2POperationOutput::PeerConnected(peer_entity)).await;
                    }));

                    continue;
                };

                let request_id = msg.request_id;
                if let Some(response) = msg.response {
                    self.shared_context.notify_peer_response(&peer_id, request_id.clone(), response).await;
                    continue;
                };

                let Some(request) = msg.request else {
                    continue;
                };

                if let Some(peer) = self.shared_context.get_peer(&peer_id).await.and_then(|it| it.upgrade()) {
                    peer.process_message_packet(request_id, request).await;
                };
            }

            for (peer_id, data) in socket.channel_mut(TRANSFER_RESOURCE_RELIABLE_CHANNEL_ID).receive() {
                let Some(peer) = self.shared_context.get_peer(&peer_id).await.and_then(|it| it.upgrade()) else {
                    continue;
                };

                peer.process_data_packet(data).await;
            }

            for (peer_id, data) in socket.channel_mut(TRANSFER_RESOURCE_UNRELIABLE_CHANNEL_ID).receive() {
                let Some(peer) = self.shared_context.get_peer(&peer_id).await.and_then(|it| it.upgrade()) else {
                    continue;
                };

                peer.process_data_packet(data).await;
            }

            for (peer_id, data) in socket.channel_mut(TRANSFER_RESOURCE2_UNRELIABLE_CHANNEL_ID).receive() {
                let Some(peer) = self.shared_context.get_peer(&peer_id).await.and_then(|it| it.upgrade()) else {
                    continue;
                };

                peer.process_data_packet(data).await;
            }

            for (peer_id, data) in socket.channel_mut(TRANSFER_THUMBNAIL_CHANNEL_ID).receive() {
                let Some(peer) = self.shared_context.get_peer(&peer_id).await.and_then(|it| it.upgrade()) else {
                    continue;
                };

                peer.process_thumbnail_packet(data).await;
            }

            select! {
                _ = (&mut timeout).fuse() => {
                    timeout.reset(Duration::from_millis(5));
                }
                result = &mut loop_fut => {
                    break result;
                }
            }
        };

        log::info!("Stopping WebRTC server, loop stopped");
        socket.close();
        self.is_running.store(false, std::sync::atomic::Ordering::SeqCst);
        if let Err(err) = result {
            let web_rtc_errors = WebRtcErrors::SignallingClientError(anyhow!("WebRTC loop failed: {err}"));
            core_request.response(CoreOperationOutput::Error(web_rtc_errors.into())).await;
            return Ok(())
        };

        core_request.response(P2POperationOutput::NearbyServerStopped).await;

        Ok(())
    }
}

impl Drop for WebRtc {
    fn drop(&mut self) {
        block_on(async {
            self.stop().await;
        });
    }
}
