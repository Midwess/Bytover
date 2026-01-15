use crate::app::operations::p2p::P2POperationOutput;
use crate::app::operations::CoreOperationOutput;
use crate::entities::finding_scope::FindingScope;
use crate::entities::local_resource::LocalResource;
use crate::entities::peer::Peer as PeerEntity;
use crate::entities::transfer_session::TransferProgress;
use crate::errors::CoreError;
use crate::protocol::webrtc::errors::WebRtcErrors;
use crate::protocol::webrtc::fec::{CHUNK_SIZE, DATA_SHARDS_DEFAULT};
use crate::protocol::webrtc::message_channel::DirectMessageChannel;
use crate::protocol::webrtc::peer::WebRtcPeer;
use crate::protocol::webrtc::quad_channel::QuadUnreliableChannel;
use crate::protocol::webrtc::signalling::{SharedContext, WebSignallerBuilder};
use crate::repository::local_resource::LocalResourceRepository;
use crate::repository::transfer_session::TransferSessionRepository;
use crate::shell::api::CoreRequest;
use anyhow::anyhow;
use futures::executor::block_on;
use futures::select;
use futures_timer::Delay;
use futures_util::FutureExt;
use matchbox_protocol::PeerId;
use matchbox_socket::{ChannelConfig, WebRtcSocket};
use n0_future::task::spawn;
use n0_future::time::sleep;
use prost::Message;
use schema::devlog::bitbridge::peer_message_body::Request;
use schema::devlog::bitbridge::PeerMessageBody;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::Arc;
use std::time::Duration;

pub static MSG_CHANNEL_ID: usize = 0;
pub static TRANSFER_RESOURCE_RELIABLE_CHANNEL_ID: usize = 1;
pub static TRANSFER_RESOURCE_UNRELIABLE_CHANNEL_ID: usize = 2;
pub static UNORDERED_MSG_CHANNEL_ID: usize = 3;
pub static TRANSFER_RESOURCE2_UNRELIABLE_CHANNEL_ID: usize = 4;
pub static TRANSFER_RESOURCE3_UNRELIABLE_CHANNEL_ID: usize = 5;
pub static TRANSFER_RESOURCE4_UNRELIABLE_CHANNEL_ID: usize = 6;

pub static MAX_NUM_BLOCK: usize = 2;
pub static MAX_BUFFER_SIZE: usize = MAX_NUM_BLOCK * CHUNK_SIZE * DATA_SHARDS_DEFAULT;
pub static MIN_BUFFER_SIZE: usize = CHUNK_SIZE;

pub struct WebRtc {
    addr: String,
    local_resource_repository: Arc<dyn LocalResourceRepository>,
    transfer_session_repo: Arc<dyn TransferSessionRepository>,
    shared_context: SharedContext,
    is_running: AtomicBool
}

impl WebRtc {
    pub fn new(
        addr: String,
        local_resource_repository: Arc<dyn LocalResourceRepository>,
        transfer_session_repo: Arc<dyn TransferSessionRepository>
    ) -> Self {
        Self {
            addr,
            local_resource_repository,
            transfer_session_repo,
            shared_context: SharedContext::new(),
            is_running: AtomicBool::new(false)
        }
    }

    pub fn is_running(&self) -> bool {
        self.is_running.load(SeqCst)
    }

    pub async fn stop(&self) {
        let is_running = self.is_running.load(SeqCst);
        if !is_running {
            return;
        }

        self.is_running.store(false, SeqCst);
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

    pub async fn cancel_resource(&self, peer_id: String, session_id: u64, resource_id: u64) -> Result<(), WebRtcErrors> {
        let peer_id = PeerId(peer_id.parse()?);
        if let Some(peer) = self.shared_context.get_peer(&peer_id).await.and_then(|peer| peer.upgrade()) {
            peer.cancel_resource_transfer(session_id, resource_id).await;
            return Ok(())
        };

        Err(WebRtcErrors::ConnectionNotFound(peer_id))
    }

    pub async fn broadcast_cancel_session(&self, session_id: u64, resource_id: Option<u64>) -> Result<(), WebRtcErrors> {
        let peers = self.shared_context.get_all_connected_peers().await;

        log::info!("Broadcasting cancel for session {} to {} peers", session_id, peers.len());

        for peer in peers {
            if let Some(resource_id) = resource_id {
                peer.cancel_resource_transfer(session_id, resource_id).await;
            } else {
                peer.cancel_transfer(session_id).await;
            }
        }

        Ok(())
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

    pub async fn view_session_detail(
        &self,
        peer_id: String,
        request: CoreRequest,
        order_id: u64,
        password: Option<String>
    ) -> Result<(), WebRtcErrors> {
        let peer_id = PeerId(peer_id.parse()?);
        if let Some(peer) = self.shared_context.get_peer(&peer_id).await.and_then(|p| p.upgrade()) {
            peer.request_session_detail(request, order_id, password).await?;
            // Session is emitted via core_request in the peer method
            Ok(())
        } else {
            Err(WebRtcErrors::ConnectionNotFound(peer_id))
        }
    }

    pub async fn send_session_detail(
        &self,
        peer_id: String,
        request_id: String,
        session_message: Option<schema::devlog::bitbridge::P2pTransferSessionMessage>,
        resources: Option<Vec<LocalResource>>,
        error: Option<CoreError>
    ) -> Result<(), WebRtcErrors> {
        let peer_id = PeerId(peer_id.parse()?);
        if let Some(peer) = self.shared_context.get_peer(&peer_id).await.and_then(|p| p.upgrade()) {
            peer.send_session_detail_response(request_id, session_message, resources, error).await
        } else {
            Err(WebRtcErrors::ConnectionNotFound(peer_id))
        }
    }

    pub async fn download_resource(
        &self,
        peer_id: String,
        request: CoreRequest,
        session_order_id: u64,
        resource: LocalResource,
        progress: TransferProgress
    ) -> Result<(), WebRtcErrors> {
        let peer_id = PeerId(peer_id.parse()?);
        if let Some(peer) = self.shared_context.get_peer(&peer_id).await.and_then(|p| p.upgrade()) {
            peer.request_resource_download(request, session_order_id, resource, progress).await
        } else {
            Err(WebRtcErrors::ConnectionNotFound(peer_id))
        }
    }

    pub async fn download_all_resources(
        &self,
        peer_id: String,
        request: CoreRequest,
        session_order_id: u64,
        session_resource: LocalResource,
        resources: Vec<LocalResource>,
        _aggregate_progress: TransferProgress
    ) -> Result<(), WebRtcErrors> {
        let peer_id = PeerId(peer_id.parse()?);
        if let Some(peer) = self.shared_context.get_peer(&peer_id).await.and_then(|p| p.upgrade()) {
            peer.download_all_resources(request, session_order_id, session_resource, resources).await
        } else {
            Err(WebRtcErrors::ConnectionNotFound(peer_id))
        }
    }

    pub async fn stream_resource_to_peer(
        &self,
        peer_id: String,
        session_id: u64,
        transfer_id: u16,
        resource: LocalResource
    ) -> Result<(), WebRtcErrors> {
        let peer_id = PeerId(peer_id.parse()?);
        if let Some(peer) = self.shared_context.get_peer(&peer_id).await.and_then(|p| p.upgrade()) {
            peer.stream_resource(session_id, transfer_id, resource).await
        } else {
            Err(WebRtcErrors::ConnectionNotFound(peer_id))
        }
    }

    pub async fn send_resource_notification(
        &self,
        peer_id: String,
        session_id: u64,
        resource: LocalResource
    ) -> Result<(), WebRtcErrors> {
        let peer_id = PeerId(peer_id.parse()?);
        if let Some(peer) = self.shared_context.get_peer(&peer_id).await.and_then(|p| p.upgrade()) {
            peer.send_resource_notification(session_id, resource).await
        } else {
            Err(WebRtcErrors::ConnectionNotFound(peer_id))
        }
    }

    pub async fn start(&self, core_request: CoreRequest, current_user: PeerEntity) -> Result<(), WebRtcErrors> {
        if self.is_running() {
            log::info!("The webrtc server is already running");
            core_request.response(P2POperationOutput::AlreadyRunning).await;
            return Ok(())
        }

        self.is_running.store(true, std::sync::atomic::Ordering::SeqCst);
        self.shared_context.set_current_id(current_user.peer_id()).await;
        self.shared_context.set_core_request(core_request.clone()).await;
        log::info!("Starting WebRTC server with my peer = {current_user:?}");
        let signaller_builder = Arc::new(WebSignallerBuilder::new(self.shared_context.clone()));
        let (mut socket, loop_fut) = WebRtcSocket::builder(self.addr.clone())
            .signaller_builder(signaller_builder.clone())
            .add_reliable_channel(Some(MIN_BUFFER_SIZE))
            .add_channel(ChannelConfig {
                buffer_low_threshold: Some(MIN_BUFFER_SIZE),
                ordered: false,
                max_retransmits: None
            })
            .add_unreliable_channel(Some(MIN_BUFFER_SIZE))
            .add_channel(ChannelConfig {
                buffer_low_threshold: Some(MIN_BUFFER_SIZE),
                ordered: false,
                max_retransmits: None
            })
            .add_unreliable_channel(Some(MIN_BUFFER_SIZE))
            .add_unreliable_channel(Some(MIN_BUFFER_SIZE))
            .add_unreliable_channel(Some(MIN_BUFFER_SIZE))
            .signaling_keep_alive_interval(Some(Duration::from_millis(5000)))
            .reconnect_attempts(Some(u16::MAX))
            .relay_fallback_on_timeout(true)
            .handshake_timeout(Duration::from_secs(15))
            .build();

        let loop_fut = loop_fut.fuse();
        futures::pin_mut!(loop_fut);
        let timeout = Delay::new(Duration::from_millis(8));
        futures::pin_mut!(timeout);

        let outbound_msg_sender = socket.channel(MSG_CHANNEL_ID).sender_clone();
        let outbound_unordered_msg_sender = socket.channel(UNORDERED_MSG_CHANNEL_ID).sender_clone();
        let outbound_reliable_data_sender = socket.channel(TRANSFER_RESOURCE_RELIABLE_CHANNEL_ID).sender_clone();
        let outbound_unreliable_data_sender = socket.channel(TRANSFER_RESOURCE_UNRELIABLE_CHANNEL_ID).sender_clone();
        let outbound_unreliable2_data_sender = socket.channel(TRANSFER_RESOURCE2_UNRELIABLE_CHANNEL_ID).sender_clone();
        let outbound_unreliable3_data_sender = socket.channel(TRANSFER_RESOURCE3_UNRELIABLE_CHANNEL_ID).sender_clone();
        let outbound_unreliable4_data_sender = socket.channel(TRANSFER_RESOURCE4_UNRELIABLE_CHANNEL_ID).sender_clone();

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
                        let unordered_msg_channel = DirectMessageChannel::new(peer_id, outbound_unordered_msg_sender.clone());
                        let current_user = current_user.clone();
                        let outbound_reliable_data_sender = outbound_reliable_data_sender.clone();
                        let local_resource_repository = self.local_resource_repository.clone();
                        let transfer_session_repo = self.transfer_session_repo.clone();
                        let context = self.shared_context.clone();
                        let core_request = core_request.clone();
                        let quad_channel = QuadUnreliableChannel::new(
                            outbound_unreliable_data_sender.clone(),
                            outbound_unreliable2_data_sender.clone(),
                            outbound_unreliable3_data_sender.clone(),
                            outbound_unreliable4_data_sender.clone(),
                            TRANSFER_RESOURCE_UNRELIABLE_CHANNEL_ID,
                            TRANSFER_RESOURCE2_UNRELIABLE_CHANNEL_ID,
                            TRANSFER_RESOURCE3_UNRELIABLE_CHANNEL_ID,
                            TRANSFER_RESOURCE4_UNRELIABLE_CHANNEL_ID,
                            buffer.clone()
                        );
                        self.shared_context.add_peer_msg_channel(&peer_id, &direct_message_channel).await;
                        handles.push(spawn(async move {
                            let peer = match WebRtcPeer::new(
                                current_user.clone(),
                                direct_message_channel,
                                unordered_msg_channel,
                                outbound_reliable_data_sender,
                                quad_channel,
                                buffer,
                                local_resource_repository,
                                transfer_session_repo
                            )
                            .await
                            {
                                Ok(peer) => peer,
                                Err(err) => {
                                    log::error!("Failed to connect to peer {err:?}");
                                    return;
                                }
                            };

                            let peer = Arc::new(peer);

                            let peer_entity = peer.peer.clone();
                            context.add_peer(Arc::downgrade(&peer)).await;
                            let _ = core_request.response(P2POperationOutput::PeerConnected(peer_entity)).await;
                            let result = peer.run_loop().await;
                            log::info!("Peer {peer_id} loop finished with result {result:?}");
                            context.remove_peer(&peer_id).await;
                        }));
                    }
                } else if state == matchbox_socket::PeerState::Disconnected {
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
                    let unordered_msg_channel = DirectMessageChannel::new(peer_id, outbound_unordered_msg_sender.clone());
                    let current_user = current_user.clone();
                    let peer_id = peer_id;
                    let outbound_reliable_data_sender = outbound_reliable_data_sender.clone();
                    let local_resource_repository = self.local_resource_repository.clone();
                    let transfer_session_repo = self.transfer_session_repo.clone();
                    let request_id = msg.request_id.clone();
                    let core_request = core_request.clone();
                    let context = self.shared_context.clone();
                    let quad_channel = QuadUnreliableChannel::new(
                        outbound_unreliable_data_sender.clone(),
                        outbound_unreliable2_data_sender.clone(),
                        outbound_unreliable3_data_sender.clone(),
                        outbound_unreliable4_data_sender.clone(),
                        TRANSFER_RESOURCE_UNRELIABLE_CHANNEL_ID,
                        TRANSFER_RESOURCE2_UNRELIABLE_CHANNEL_ID,
                        TRANSFER_RESOURCE3_UNRELIABLE_CHANNEL_ID,
                        TRANSFER_RESOURCE4_UNRELIABLE_CHANNEL_ID,
                        buffer.clone()
                    );
                    context.add_peer_msg_channel(&peer_id, &direct_message_channel).await;
                    handles.push(spawn(async move {
                        let peer = match WebRtcPeer::from_introduce_request(
                            current_user,
                            request_id,
                            request,
                            direct_message_channel,
                            unordered_msg_channel,
                            outbound_reliable_data_sender,
                            quad_channel,
                            buffer,
                            local_resource_repository,
                            transfer_session_repo
                        )
                        .await
                        {
                            Ok(peer) => peer,
                            Err(err) => {
                                log::error!("Failed to connect to peer {err:?}");
                                return;
                            }
                        };

                        let peer = Arc::new(peer);
                        let peer_entity = peer.peer.clone();
                        context.add_peer(Arc::downgrade(&peer)).await;
                        let _ = core_request.response(P2POperationOutput::PeerConnected(peer_entity)).await;
                        let result = peer.run_loop().await;
                        log::info!("Peer {peer_id} loop finished with result {result:?}");
                        context.remove_peer(&peer_id).await;
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

            for (peer_id, data) in socket.channel_mut(TRANSFER_RESOURCE3_UNRELIABLE_CHANNEL_ID).receive() {
                let Some(peer) = self.shared_context.get_peer(&peer_id).await.and_then(|it| it.upgrade()) else {
                    continue;
                };

                peer.process_data_packet(data).await;
            }

            for (peer_id, data) in socket.channel_mut(TRANSFER_RESOURCE4_UNRELIABLE_CHANNEL_ID).receive() {
                let Some(peer) = self.shared_context.get_peer(&peer_id).await.and_then(|it| it.upgrade()) else {
                    continue;
                };

                peer.process_data_packet(data).await;
            }

            for (peer_id, msg) in socket.channel_mut(UNORDERED_MSG_CHANNEL_ID).receive() {
                let Ok(msg) = PeerMessageBody::decode(&msg[..]) else {
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
