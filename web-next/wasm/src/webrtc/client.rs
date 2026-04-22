use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use core_services::utils::cancellation::FutureExtension;
use core_services::utils::yield_container::YieldContainer;
use futures::channel::mpsc::unbounded;
use futures::channel::{mpsc, oneshot};
use futures::select_biased;
use futures::stream::StreamExt;
use futures_util::lock::Mutex;
use futures_util::{FutureExt, SinkExt};
use once_cell::sync::OnceCell;
use schema::devlog::bitbridge::peer_message_body::{Request, Response};
use schema::devlog::bitbridge::{P2pCancelSessionRequest, ResourceTypeMessage, VoidResponseMessage};
use shared::app::operations::p2p::P2POperationOutput;
use shared::app::operations::transfer::TransferOperationOutput;
use shared::app::operations::CoreOperationOutput;
use shared::entities::local_resource::{LocalResource, LocalResourcePath, ResourceType};
use shared::entities::peer::Peer as PeerEntity;
use shared::entities::transfer_session::TransferProgress;
use shared::protocol::webrtc::errors::WebRtcErrors;
use shared::protocol::webrtc::message_channel::DirectMessageChannel;
use shared::protocol::webrtc::packet::WebRtcPacket;
use shared::protocol::webrtc::transfer::{TransferDelimiterShema, TransfersContext};
use shared::repository::local_resource::LocalResourceRepository;
use shared::repository::transfer_session::TransferSessionRepository;
use shared::shell::api::CoreRequest;

use crate::webrtc::ice::IceAgent;
use crate::webrtc::signaling::SignalingClient;
use crate::webrtc::web::channel_ids::*;
use crate::webrtc::web::{RtcConnectionWrapper, RtcDataChannelWrapper, WebRtcApi};

pub type WebRtcClientError = WebRtcErrors;

struct ReassemblyEntry {
    buf: Vec<u8>,
    received: u32,
    part_count: u8,
    final_len: usize,
}

impl ReassemblyEntry {
    fn new(part_count: u8) -> Self {
        debug_assert!(part_count > 0 && part_count < 32);
        let cap = part_count as usize * WIRE_PART_SIZE;
        let mut buf = Vec::with_capacity(cap);
        unsafe {
            buf.set_len(cap);
        }
        Self {
            buf,
            received: 0,
            part_count,
            final_len: 0,
        }
    }

    fn insert(&mut self, part_index: u8, payload: &[u8]) -> bool {
        debug_assert!(part_index < self.part_count);
        debug_assert!(payload.len() <= WIRE_PART_SIZE);
        debug_assert!(part_index + 1 == self.part_count || payload.len() == WIRE_PART_SIZE);

        let bit = 1u32 << part_index;
        if self.received & bit != 0 {
            return false;
        }
        let dst_offset = part_index as usize * WIRE_PART_SIZE;
        unsafe {
            std::ptr::copy_nonoverlapping(
                payload.as_ptr(),
                self.buf.as_mut_ptr().add(dst_offset),
                payload.len(),
            );
        }
        self.received |= bit;
        if part_index + 1 == self.part_count {
            self.final_len = dst_offset + payload.len();
        }
        self.received == (1u32 << self.part_count) - 1
    }

    fn finalize(mut self) -> Vec<u8> {
        unsafe {
            self.buf.set_len(self.final_len);
        }
        self.buf
    }
}

pub struct ConnectionSlot {
    pub index: usize,
    pub connection: Arc<RtcConnectionWrapper>,
    pub reliable_channel: Arc<RtcDataChannelWrapper>,
}

pub struct WebRtcClient {
    transfers_context: TransfersContext,
    me: OnceCell<PeerEntity>,
    peer: OnceCell<PeerEntity>,
    core_request: OnceCell<CoreRequest>,
    resource_repo: Arc<dyn LocalResourceRepository>,
    transfer_session_repo: Arc<dyn TransferSessionRepository>,
    inbound_msg_receiver: YieldContainer<mpsc::UnboundedReceiver<Vec<u8>>>,
    inbound_data_receiver: YieldContainer<mpsc::UnboundedReceiver<Vec<u8>>>,

    msg_channel: OnceCell<DirectMessageChannel>,
    unordered_msg_channel: OnceCell<DirectMessageChannel>,
    prefix_channels: Mutex<HashMap<u16, mpsc::Sender<(u64, Vec<u8>)>>>,
    connections: Mutex<Vec<ConnectionSlot>>,
    disconnect_signal: Mutex<Option<oneshot::Sender<()>>>,
    disconnect_receiver: YieldContainer<oneshot::Receiver<()>>,
    disconnect_requested: AtomicBool,
}

fn spawn_outbound_sender(channel: Arc<RtcDataChannelWrapper>, mut rx: mpsc::Receiver<Vec<u8>>) {
    wasm_bindgen_futures::spawn_local(async move {
        while let Some(data) = rx.next().await {
            let arr = js_sys::Uint8Array::from(&data[..]);
            let p2p_open = channel.0.ready_state() == web_sys::RtcDataChannelState::Open;

            if p2p_open {
                let _ = channel.send_with_array_buffer_view(&arr);
            } else {
                log::warn!("P2P data channel is closed, discarding packet");
            }
        }
    });
}

impl WebRtcClient {
    pub async fn connect(
        me: PeerEntity,
        signaling: SignalingClient,
        ice_agent: IceAgent,
        peer_id: &str,
        resource_repo: Arc<dyn LocalResourceRepository>,
        transfer_session_repo: Arc<dyn TransferSessionRepository>,
    ) -> Result<Arc<Self>, WebRtcClientError> {
        log::info!("WebRtcClient connecting to peer {}", peer_id);

        let ice_configs = signaling
            .fetch_relay_configs(peer_id)
            .await
            .unwrap_or_else(|e| {
                log::warn!("Failed to fetch relay configs: {:?}", e);
                vec![schema::devlog::rpc_signalling::server::IceConfig {
                    urls: vec!["stun:stun.l.google.com:19302".to_string()],
                    ..Default::default()
                }]
            });

        let total_slots = ice_configs.len().max(1);
        log::info!("Using {} ice config(s) (total_slots={})", ice_configs.len(), total_slots);

        let primary_config = ice_configs
            .first()
            .cloned()
            .unwrap_or_else(|| schema::devlog::rpc_signalling::server::IceConfig {
                urls: vec!["stun:stun.l.google.com:19302".to_string()],
                ..Default::default()
            });
        let session_id = uuid::Uuid::new_v4().to_string();

        let api = WebRtcApi::new(primary_config.clone());
        let connection = api.create_peer_connection().map_err(|e| WebRtcClientError::Connection(e.to_string()))?;

        let (msg_inbound_tx, msg_inbound_rx) = unbounded();
        let (data_inbound_tx, data_inbound_rx) = unbounded();
        let (ordered_out_tx, ordered_out_rx) = mpsc::channel(16);
        let (unordered_out_tx, unordered_out_rx) = mpsc::channel(16);
        let (disconnect_tx, disconnect_rx) = oneshot::channel();

        let reliable_channel = api
            .create_unordered_channel(connection.clone(), RELIABLE_DATA_CHANNEL_ID)
            .map_err(|e| WebRtcClientError::Connection(e.to_string()))?;
        let unordered_channel = api
            .create_unordered_channel(connection.clone(), UNORDERED_MSG_CHANNEL_ID)
            .map_err(|e| WebRtcClientError::Connection(e.to_string()))?;
        let ordered_channel = api
            .create_ordered_channel(connection.clone(), ORDERED_MSG_CHANNEL_ID)
            .map_err(|e| WebRtcClientError::Connection(e.to_string()))?;

        api.setup_channel_handlers(reliable_channel.clone(), data_inbound_tx.clone())
            .map_err(|e| WebRtcClientError::Connection(e.to_string()))?;
        api.setup_channel_handlers(unordered_channel.clone(), msg_inbound_tx.clone())
            .map_err(|e| WebRtcClientError::Connection(e.to_string()))?;
        api.setup_channel_handlers(ordered_channel.clone(), msg_inbound_tx.clone())
            .map_err(|e| WebRtcClientError::Connection(e.to_string()))?;

        api.create_offer_and_set_local(&connection)
            .await
            .map_err(|e| WebRtcClientError::Connection(e.to_string()))?;

        ice_agent
            .wait_for_gathering_complete(&connection)
            .await
            .map_err(|e| WebRtcClientError::Connection(e.to_string()))?;

        let local_sdp = connection
            .local_description()
            .ok_or_else(|| WebRtcClientError::Connection("No P2P local description".to_string()))?
            .sdp();

        log::info!("ICE gathering complete, SDP ready. Starting signalling.");

        let (open_tx, mut open_rx) = mpsc::channel::<()>(1);

        let p2p_tx = open_tx.clone();
        let sig_p2p = signaling.clone();
        let peer_id_p2p = peer_id.to_string();
        let session_id_p2p = session_id.clone();
        let local_sdp_p2p = local_sdp.clone();
        let connection_p2p = connection.clone();
        let reliable_channel_p2p = reliable_channel.clone();
        let unordered_channel_p2p = unordered_channel.clone();
        let ordered_channel_p2p = ordered_channel.clone();

        let me_proto = schema::devlog::bitbridge::PeerMessage::from(me.clone());

        let client = Arc::new(WebRtcClient {
            transfers_context: TransfersContext::new(),
            me: OnceCell::new(),
            peer: OnceCell::new(),
            core_request: OnceCell::new(),
            resource_repo,
            transfer_session_repo,
            inbound_msg_receiver: YieldContainer::new(msg_inbound_rx),
            inbound_data_receiver: YieldContainer::new(data_inbound_rx),
            msg_channel: OnceCell::new(),
            unordered_msg_channel: OnceCell::new(),
            prefix_channels: Mutex::new(HashMap::new()),
            connections: Mutex::new(Vec::with_capacity(total_slots)),
            disconnect_signal: Mutex::new(Some(disconnect_tx)),
            disconnect_receiver: YieldContainer::new(disconnect_rx),
            disconnect_requested: AtomicBool::new(false),
        });

        let _ = client.me.set(me.clone());

        let p2p_res = sig_p2p
            .send_offer(&peer_id_p2p, &local_sdp_p2p, &session_id_p2p, me_proto.clone(), 0)
            .await;
        let mut p2p_answer_sdp = None;
        if let Ok((answer_sdp, remote_peer_proto)) = p2p_res {
            log::info!("Got P2P answer from remote peer {answer_sdp:?}");

            let remote_peer = PeerEntity::from(remote_peer_proto);

            let _ = client.peer.set(remote_peer);
            p2p_answer_sdp = Some(answer_sdp);
        } else {
            log::warn!("P2P signalling failed: {:?}", p2p_res);
        }

        wasm_bindgen_futures::spawn_local(async move {
            if let Some(answer_sdp) = p2p_answer_sdp {
                if let Err(e) = api.set_remote_description(&connection_p2p, &answer_sdp).await {
                    log::warn!("p2p remote desc failed {:?}", e);
                }
            }
            let _ = api.wait_for_channel_open(reliable_channel_p2p).await;
            let _ = api.wait_for_channel_open(unordered_channel_p2p).await;
            if api.wait_for_channel_open(ordered_channel_p2p).await.is_ok() {
                log::info!("[webrtc-client] slot 0 open!");
                let _ = p2p_tx.clone().send(()).await;
            }
        });

        client.connections.lock().await.push(ConnectionSlot {
            index: 0,
            connection: connection.clone(),
            reliable_channel: reliable_channel.clone(),
        });

        let _ = client.msg_channel.set(DirectMessageChannel::new(ordered_out_tx));
        let _ = client.unordered_msg_channel.set(DirectMessageChannel::new(unordered_out_tx));

        spawn_outbound_sender(ordered_channel.clone(), ordered_out_rx);
        spawn_outbound_sender(unordered_channel.clone(), unordered_out_rx);

        let _ = open_rx.next().await;

        log::info!("WebRtcClient slot 0 established; peer info exchanged via signaling.");

        for (i, slot_config) in ice_configs.into_iter().enumerate().skip(1) {
            let sig = signaling.clone();
            let ice_agent = ice_agent.clone();
            let peer_id_owned = peer_id.to_string();
            let session_id_owned = session_id.clone();
            let data_inbound_tx = data_inbound_tx.clone();
            let me_proto = schema::devlog::bitbridge::PeerMessage::from(me.clone());
            let client_weak = Arc::downgrade(&client);

            wasm_bindgen_futures::spawn_local(async move {
                match Self::install_slot(i, slot_config, sig, ice_agent, peer_id_owned, session_id_owned, me_proto, data_inbound_tx)
                    .await
                {
                    Ok(slot) => {
                        if let Some(client) = client_weak.upgrade() {
                            client.connections.lock().await.push(slot);
                            log::info!("[webrtc-client] slot {i} joined pool");
                        }
                    }
                    Err(e) => {
                        log::warn!("[webrtc-client] slot {i} failed to connect: {e:?}");
                    }
                }
            });
        }

        Ok(client)
    }

    async fn install_slot(
        index: usize,
        ice_config: schema::devlog::rpc_signalling::server::IceConfig,
        signaling: SignalingClient,
        ice_agent: IceAgent,
        peer_id: String,
        session_id: String,
        me_proto: schema::devlog::bitbridge::PeerMessage,
        data_inbound_tx: mpsc::UnboundedSender<Vec<u8>>,
    ) -> Result<ConnectionSlot, WebRtcClientError> {
        log::info!("[webrtc-client] Installing slot {index}");

        let api = WebRtcApi::new(ice_config);
        let connection = api
            .create_peer_connection()
            .map_err(|e| WebRtcClientError::Connection(e.to_string()))?;

        let reliable_channel = api
            .create_unordered_channel(connection.clone(), RELIABLE_DATA_CHANNEL_ID)
            .map_err(|e| WebRtcClientError::Connection(e.to_string()))?;

        api.setup_channel_handlers(reliable_channel.clone(), data_inbound_tx)
            .map_err(|e| WebRtcClientError::Connection(e.to_string()))?;

        api.create_offer_and_set_local(&connection)
            .await
            .map_err(|e| WebRtcClientError::Connection(e.to_string()))?;

        ice_agent
            .wait_for_gathering_complete(&connection)
            .await
            .map_err(|e| WebRtcClientError::Connection(e.to_string()))?;

        let local_sdp = connection
            .local_description()
            .ok_or_else(|| WebRtcClientError::Connection(format!("No local description for slot {index}")))?
            .sdp();

        let slot_idx = u32::try_from(index).unwrap_or(u32::MAX);
        let (answer_sdp, _remote_peer) = signaling
            .send_offer(&peer_id, &local_sdp, &session_id, me_proto, slot_idx)
            .await
            .map_err(|e| WebRtcClientError::Signalling(format!("slot {index} signalling failed: {e:?}")))?;

        api.set_remote_description(&connection, &answer_sdp)
            .await
            .map_err(|e| WebRtcClientError::Connection(format!("slot {index} remote desc failed: {e:?}")))?;

        api.wait_for_channel_open(reliable_channel.clone())
            .await
            .map_err(|e| WebRtcClientError::Connection(format!("slot {index} channel never opened: {e:?}")))?;

        Ok(ConnectionSlot {
            index,
            connection,
            reliable_channel,
        })
    }

    pub async fn run(self: Arc<Self>) -> Result<(), WebRtcClientError> {
        log::info!("WebRtcClient run loop starting");

        let mut disconnect_receiver_guard = self.disconnect_receiver.retrieve().await?;
        let disconnect_receiver = disconnect_receiver_guard
            .value
            .take()
            .ok_or_else(|| WebRtcClientError::Connection("Disconnect receiver already taken".to_string()))?;

        let msg_future = self.message_loop();
        let data_future = self.data_receiving_loop();
        let disconnect_future = async move {
            let _ = disconnect_receiver.await;
            Ok::<(), WebRtcClientError>(())
        };

        futures::pin_mut!(msg_future, data_future, disconnect_future);
        select_biased! {
            r = disconnect_future.fuse() => r?,
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
        let _ = self.me.set(current_user.clone());

        let introduce_request = schema::devlog::bitbridge::IntroduceRequestMessage {
            mine: schema::devlog::bitbridge::PeerMessage::from(current_user.clone()),
        };

        let msg_channel = self
            .msg_channel
            .get()
            .ok_or_else(|| WebRtcClientError::Connection("No message channel".to_string()))?;

        let response = msg_channel.send(Request::IntroduceRequest(introduce_request), None).await?;

        match response {
            Response::IntroduceResponse(resp) => {
                let peer = PeerEntity::from(resp.peer);
                let _ = self.set_peer(peer);
                log::info!("Introduce handshake completed");
                Ok(())
            }
            _ => Err(WebRtcClientError::InvalidResponse("Unexpected response type".to_string())),
        }
    }

    pub async fn request_session_detail(
        &self,
        core_request: CoreRequest,
        order_id: u64,
        password: Option<String>,
    ) -> Result<(), WebRtcClientError> {
        use core_services::utils::cancellation::CancellationToken;
        use schema::devlog::bitbridge::view_session_detail_response::Result as ResponseResult;
        use schema::devlog::bitbridge::PeerErrorsMessage;

        let request = schema::devlog::bitbridge::ViewSessionDetailRequest { order_id, password };

        let msg_channel = self
            .msg_channel
            .get()
            .ok_or_else(|| WebRtcClientError::Connection("No message channel".to_string()))?;

        let timeout_token = CancellationToken::timeout(Duration::from_secs(60));

        let response = msg_channel
            .send(Request::ViewSessionRequest(request), None)
            .with_cancel(&timeout_token)
            .await
            .map_err(|_| WebRtcClientError::Timeout)??;

        match response {
            Response::ViewSessionResponse(resp) => match resp.result {
                Some(ResponseResult::Session(session)) => {
                    core_request
                        .response(CoreOperationOutput::Transfer(TransferOperationOutput::SessionDetailReceived(
                            session,
                        )))
                        .await;
                }
                Some(ResponseResult::Error(error_type)) => {
                    let error_msg = PeerErrorsMessage::try_from(error_type).unwrap_or(PeerErrorsMessage::InvalidRequest);
                    core_request
                        .response(CoreOperationOutput::Error(shared::errors::CoreError::PeerRequestError(
                            error_msg,
                        )))
                        .await;
                    return Err(WebRtcClientError::PeerError(error_msg.to_string()));
                }
                _ => return Err(WebRtcClientError::InvalidResponse("Unexpected response".to_string())),
            },
            _ => return Err(WebRtcClientError::InvalidResponse("Unexpected response type".to_string())),
        }

        Ok(())
    }

    pub async fn request_resource_download(
        &self,
        core_request: CoreRequest,
        session_order_id: u64,
        resource: LocalResource,
        mut progress: TransferProgress,
    ) -> Result<TransferProgress, WebRtcClientError> {
        use schema::devlog::bitbridge::DownloadResourceRequest;
        use std::sync::atomic::{AtomicU16, Ordering};

        static TRANSFER_ID_COUNTER: AtomicU16 = AtomicU16::new(1);

        let resource_order_id = resource.order_id;
        let transfer_id = TRANSFER_ID_COUNTER.fetch_add(1, Ordering::SeqCst);

        let request = DownloadResourceRequest {
            session_order_id,
            resource_order_id,
            transfer_id: transfer_id as u32,
        };

        let (tx, mut rx) = mpsc::channel::<(u64, Vec<u8>)>(10);
        self.prefix_channels.lock().await.insert(transfer_id, tx);

        let resource_token = self.transfers_context.get_or_create_resource_token(session_order_id, resource_order_id).await;

        progress.update_progress(1);
        core_request
            .response(TransferOperationOutput::TransferResourceProgressUpdate(progress.clone()))
            .await;

        let msg_channel = self
            .msg_channel
            .get()
            .ok_or_else(|| WebRtcClientError::Connection("No message channel".to_string()))?;

        msg_channel.notify(Request::DownloadResourceRequest(request)).await?;

        let start_delimiter = loop {
            match rx.next().with_cancel(&resource_token).await? {
                Some((_, packet)) => {
                    if let Ok(v) = TransferDelimiterShema::from_start_packet(&packet, session_order_id) {
                        break v;
                    }
                }
                None => {
                    self.prefix_channels.lock().await.remove(&transfer_id);
                    return Err(WebRtcClientError::Transfer("Channel closed before start delimiter".into()));
                }
            }
        };

        let compressed = start_delimiter.compressed();
        let mut writer = self.resource_repo.write(resource.path.clone(), compressed).await?;

        let mut expected_size: Option<u64> = None;
        loop {
            match rx.next().with_cancel(&resource_token).await? {
                Some((offset, packet)) => {
                    if let Ok(delimiter) = TransferDelimiterShema::from_bytes(&packet) {
                        if matches!(delimiter, TransferDelimiterShema::End { .. }) && delimiter.session_id() == Some(session_order_id)
                        {
                            expected_size = delimiter.total_size();

                            if let Some(target) = expected_size {
                                if progress.total_bytes() >= target {
                                    progress.success();
                                    core_request
                                        .response(TransferOperationOutput::TransferResourceProgressUpdate(progress.clone()))
                                        .await;
                                    break;
                                }
                            } else {
                                progress.success();
                                core_request
                                    .response(TransferOperationOutput::TransferResourceProgressUpdate(progress.clone()))
                                    .await;
                                break;
                            }
                            continue;
                        }
                    }

                    // Stop tracking transfer if context is cancelled
                    if resource_token.is_cancelled() {
                        break;
                    }

                    let bytes = Bytes::from(packet.to_vec());
                    if let Some(written) =
                        writer.d_write_at(bytes, offset).await.map_err(|e| WebRtcClientError::Transfer(e.to_string()))?
                    {
                        progress.update_progress(written as u64);
                        core_request
                            .response_throttle(TransferOperationOutput::TransferResourceProgressUpdate(progress.clone()))
                            .await;

                        if let Some(target) = expected_size {
                            if progress.total_bytes() >= target {
                                progress.success();
                                core_request
                                    .response(TransferOperationOutput::TransferResourceProgressUpdate(progress.clone()))
                                    .await;
                                break;
                            }
                        }
                    }
                }
                None => {
                    self.prefix_channels.lock().await.remove(&transfer_id);
                    return Err(WebRtcClientError::Transfer("Channel closed before completion".into()));
                }
            }
        }

        self.prefix_channels.lock().await.remove(&transfer_id);
        Ok(progress)
    }

    pub async fn download_all_resources(
        &self,
        core_request: CoreRequest,
        session_order_id: u64,
        session_resource: LocalResource,
        resources: Vec<LocalResource>,
    ) -> Result<TransferProgress, WebRtcClientError> {
        use shared::entities::transfer_session::TransferType;

        let token = self
            .transfers_context
            .get_or_create_resource_token(session_order_id, session_resource.order_id)
            .await;
        let zip_path = session_resource.path.clone();

        if let Err(e) = self.transfer_session_repo.start_download_session(zip_path.clone()).await {
            self.cancel_resource_transfer(session_order_id, session_resource.order_id).await;
            return Err(WebRtcClientError::Transfer(format!(
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

        if download_failed {
            let _ = self.transfer_session_repo.stop_download_session(zip_path).await;
            self.cancel_resource_transfer(session_order_id, session_resource.order_id).await;
            return Err(WebRtcClientError::Transfer("Download all failed".into()));
        }

        let mut session_progress = TransferProgress::new(session_resource.order_id, session_resource.size, TransferType::Receive);
        session_progress.update_progress(session_resource.size);
        core_request
            .response(TransferOperationOutput::TransferResourceProgressUpdate(
                session_progress.clone(),
            ))
            .await;

        if let Err(e) = self.transfer_session_repo.stop_download_session(zip_path).await {
            session_progress.fail(format!("Failed to save: {:?}", e));
            core_request
                .response(TransferOperationOutput::TransferResourceProgressUpdate(session_progress))
                .await;
            return Err(WebRtcClientError::Transfer("Failed to stop download session".into()));
        }

        session_progress.success();
        core_request
            .response(TransferOperationOutput::TransferResourceProgressUpdate(
                session_progress.clone(),
            ))
            .await;

        Ok(session_progress)
    }

    pub fn start_core_stream(&self, core_request: CoreRequest) {
        let _ = self.core_request.set(core_request);
    }

    pub fn core_request(&self) -> Option<&CoreRequest> {
        self.core_request.get()
    }

    pub async fn disconnect(&self) {
        if self.disconnect_requested.swap(true, Ordering::SeqCst) {
            return;
        }

        log::info!("Disconnecting WebRtcClient from peer {:?}", self.peer_id());

        let mut slots = self.connections.lock().await;
        for slot in slots.drain(..) {
            slot.reliable_channel.close();
            slot.connection.close();
        }
        drop(slots);

        if let Some(signal) = self.disconnect_signal.lock().await.take() {
            let _ = signal.send(());
        }
    }

    pub async fn process_message_packet(&self, request_id: String, msg: Request) {
        match msg {
            Request::IntroduceRequest(introduce_msg) => {
                log::info!("Received introduce request from peer");
                let peer = PeerEntity::from(introduce_msg.mine);
                log::info!("Remote peer: {:?}", peer.id);
                let _ = self.set_peer(peer);

                if let Some(me) = self.me.get() {
                    let response = schema::devlog::bitbridge::IntroduceResponseMessage {
                        peer: schema::devlog::bitbridge::PeerMessage::from(me.clone()),
                    };
                    if let Some(msg_channel) = self.msg_channel.get() {
                        let _ = msg_channel.send_response(request_id, Response::IntroduceResponse(response)).await;
                    }
                } else {
                    log::warn!("Cannot respond to IntroduceRequest: current user not set (call introduce() first)");
                }
            }
            Request::CancelRequest(request) => {
                log::info!("Received cancel request {:?}", request);
                if let Some(resource_id) = request.resource_id {
                    self.transfers_context.cancel_resource(request.session_id, resource_id).await;
                } else {
                    self.transfers_context.cancel_transfer(request.session_id).await;
                    if let Some(core_request) = self.core_request() {
                        core_request
                            .response(CoreOperationOutput::P2P(P2POperationOutput::CancelSessionRequest {
                                session_id: request.session_id,
                            }))
                            .await;
                    }
                    self.disconnect().await;
                }
            }
            Request::ResourceNotification(notification) => {
                let session_order_id = notification.session_order_id;
                log::info!(
                    "Received resource notification for session order_id {} resource_id {:?}",
                    session_order_id,
                    notification.resource.as_ref().map(|it| it.order_id)
                );
                if let Some(resource_proto) = notification.resource {
                    let mut resource = LocalResource {
                        order_id: resource_proto.order_id,
                        name: resource_proto.name,
                        size: resource_proto.size as u64,
                        path: LocalResourcePath::RelativePath {
                            path: format!("received/session_{}/resource_{}", session_order_id, resource_proto.order_id),
                            is_private: false,
                        },
                        thumbnail_path: None,
                        r#type: (ResourceTypeMessage::try_from(resource_proto.r#type).unwrap_or_default())
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
                                log::warn!("Failed to save thumbnail: {:?}", e);
                            }
                        }
                    }

                    if let Some(core_request) = self.core_request() {
                        core_request
                            .response(CoreOperationOutput::P2P(P2POperationOutput::ReceivedResourceNotification {
                                session_order_id,
                                resource,
                                peer_id: self.peer.get().map(|p| p.id.clone()).unwrap_or_default(),
                            }))
                            .await;
                    }
                }

                if let Some(msg_channel) = self.msg_channel.get() {
                    let _ = msg_channel.send_response(request_id, Response::VoidResponse(VoidResponseMessage {})).await;
                }
            }
            _ => {
                log::debug!("Unhandled message request type");
            }
        }
    }

    pub async fn cancel_transfer(&self, session_id: u64) {
        self.transfers_context.cancel_transfer(session_id).await;
        if let Some(msg_channel) = self.msg_channel.get() {
            let cancel_msg = P2pCancelSessionRequest {
                session_id,
                resource_id: None,
            };
            let _ = msg_channel.notify(Request::CancelRequest(cancel_msg)).await;
        }
    }

    pub async fn cancel_resource_transfer(&self, session_id: u64, resource_id: u64) {
        self.transfers_context.cancel_resource(session_id, resource_id).await;
        if let Some(msg_channel) = self.msg_channel.get() {
            let cancel_msg = P2pCancelSessionRequest {
                session_id,
                resource_id: Some(resource_id),
            };
            let _ = msg_channel.notify(Request::CancelRequest(cancel_msg)).await;
        }
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
            match msg_channel.receive_packet(packet).await {
                Ok(Some(msg)) => {
                    log::debug!("message_loop: decoded request, request_id={}", msg.request_id);
                    if let Some(request) = msg.request {
                        let request_id = msg.request_id;
                        self.process_message_packet(request_id, request).await;
                    }
                }
                Ok(None) => {
                    log::info!("message_loop: decoded response (routed to pending send)");
                }
                Err(e) => {
                    log::warn!("message_loop: failed to decode packet: {:?}", e);
                }
            }
        }

        log::info!("Message channel closed, terminating message loop");
        Ok(())
    }

    async fn data_receiving_loop(&self) -> Result<(), WebRtcClientError> {
        log::info!("Starting data receiving loop");

        let mut inbound_data_receiver = self.inbound_data_receiver.retrieve().await?;

        while let Some(data) = inbound_data_receiver.next().await {
            let (prefix, offset, payload) = WebRtcPacket::deserialize(data);
            let mut channels = self.prefix_channels.lock().await;
            if let Some(tx) = channels.get_mut(&prefix) {
                if let Err(e) = tx.send((offset, payload)).await {
                    log::warn!("Prefix channel {} dropped: {:?}", prefix, e);
                    channels.remove(&prefix);
                }
            }
        }

        log::info!("Receiving loop stopped");
        Ok(())
    }
}
