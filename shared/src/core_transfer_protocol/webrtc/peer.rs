use crate::app::file_system::file::ResourceType;
use crate::app::operations::p2p::P2POperationOutput;
use crate::app::operations::transfer::TransferOperationOutput::ThumbnailFullFilled;
use crate::app::operations::CoreOperationOutput;
use crate::app::repository::errors::PersistenceError;
use crate::app::repository::local_resource::LocalResourceRepository;
use crate::app::transfer::session::{TransferSession, TransferSessionStatus};
use crate::core_api::{BufferExt, CoreBridge, TimeoutReceiver};
use crate::core_transfer_protocol::webrtc::errors::WebRtcErrors;
use crate::core_transfer_protocol::webrtc::message_channel::DirectMessageChannel;
use crate::core_transfer_protocol::webrtc::transfer::{TransferDelimiterShema, TransfersContext};
use crate::core_transfer_protocol::webrtc::webrtc::MAX_BUFFER_SIZE;
use crate::entities::peer::Peer as PeerEntity;
use futures::channel::mpsc;
use futures::lock::Mutex;
use futures_util::SinkExt;
use matchbox_protocol::PeerId;
use matchbox_socket::{Packet, PeerBuffered};
use n0_future::task::spawn;
use schema::devlog::bitbridge::peer_message_body::Response::IntroduceResponse;
use schema::devlog::bitbridge::peer_message_body::{Request, Response};
use schema::devlog::bitbridge::{
    CancelTransferSessionRequest,
    IntroduceRequestMessage,
    IntroduceResponseMessage,
    PeerMessage,
    TransferRequestMessage,
    TransferResponseMessage,
    TransferSessionMessage
};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

pub struct WebRtcPeer {
    pub msg_channel: DirectMessageChannel,
    pub peer: PeerEntity,
    pub core_bridge: Arc<dyn CoreBridge>,
    pub resource_repo: Arc<dyn LocalResourceRepository>,
    pub data_channel: Arc<Mutex<mpsc::UnboundedSender<(PeerId, Packet)>>>,
    pub thumbnail_channel: Arc<Mutex<mpsc::UnboundedSender<(PeerId, Packet)>>>,
    pub transfers_context: TransfersContext,

    pub inbound_thumbnail_stream_sender: Mutex<Option<mpsc::UnboundedSender<Packet>>>,
    pub inbound_data_stream_sender: Mutex<Option<mpsc::UnboundedSender<Packet>>>,
    pub buffer: PeerBuffered,

    pub core_id: AtomicU32
}

impl WebRtcPeer {
    pub async fn new(
        user: PeerEntity,
        msg_channel: DirectMessageChannel,
        core_bridge: Arc<dyn CoreBridge>,
        data_channel: mpsc::UnboundedSender<(PeerId, Packet)>,
        thumbnail_channel: mpsc::UnboundedSender<(PeerId, Packet)>,
        buffer: PeerBuffered,
        repository: Arc<dyn LocalResourceRepository>
    ) -> Result<Self, WebRtcErrors> {
        let introduce_request = IntroduceRequestMessage {
            mine: PeerMessage {
                peer_id: user.id().to_string(),
                name: user.name.clone(),
                avatar_url: user.avatar_url.clone(),
                device: user.device.clone().into(),
                email: user.email.clone()
            }
        };

        log::info!("Sending introduce request to other peer");
        let IntroduceResponse(response) = msg_channel.send(Request::IntroduceRequest(introduce_request), None).await? else {
            return Err(WebRtcErrors::FailedToIntroducePeer)
        };

        log::info!("Received introduce response from other peer {response:?}");

        let peer: PeerEntity = response.peer.into();

        Ok(Self {
            msg_channel,
            peer,
            data_channel: Arc::new(Mutex::new(data_channel)),
            thumbnail_channel: Arc::new(Mutex::new(thumbnail_channel)),
            core_bridge,
            transfers_context: TransfersContext::new(),
            resource_repo: repository,
            inbound_thumbnail_stream_sender: Mutex::new(None),
            inbound_data_stream_sender: Mutex::new(None),
            buffer,
            core_id: Default::default()
        })
    }

    pub async fn from_introduce_request(
        user: PeerEntity,
        request_id: String,
        msg: IntroduceRequestMessage,
        msg_channel: DirectMessageChannel,
        core_bridge: Arc<dyn CoreBridge>,
        data_channel: mpsc::UnboundedSender<(PeerId, Packet)>,
        thumbnail_channel: mpsc::UnboundedSender<(PeerId, Packet)>,
        buffer: PeerBuffered,
        repository: Arc<dyn LocalResourceRepository>
    ) -> Result<Self, WebRtcErrors> {
        log::info!("Received introduce request from other peer {:?}", msg.mine.peer_id);
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

        Ok(Self {
            msg_channel,
            peer: msg.mine.into(),
            core_bridge,
            data_channel: Arc::new(Mutex::new(data_channel)),
            thumbnail_channel: Arc::new(Mutex::new(thumbnail_channel)),
            transfers_context: TransfersContext::new(),
            resource_repo: repository,
            inbound_thumbnail_stream_sender: Mutex::new(None),
            inbound_data_stream_sender: Mutex::new(None),
            buffer,
            core_id: Default::default()
        })
    }

    pub fn start_core_stream(&self, core_stream_id: u32) {
        self.core_id.store(core_stream_id, Ordering::Relaxed);
        log::info!("Saved core stream id {}", self.core_id.load(Ordering::Relaxed));
    }

    pub async fn process_request(&self, request_id: String, msg: Request) {
        match msg {
            Request::CancelRequest(request) => {
                log::info!("Received request, cancelling transfer session {request:?}");
                self.transfers_context.stop_transfer(request.session_id as u64).await;
            }
            Request::TransferRequest(request) => {
                log::info!("Received transfer request, starting transfer session {request:?}");
                self.transfers_context.start_transfer(request.session.order_id, request_id).await;
                let response = CoreOperationOutput::P2P(P2POperationOutput::ReceivedSessionRequest {
                    remote_session: request.session
                });

                let _ = self.core_bridge.response(self.core_id.load(Ordering::Relaxed), response).await;
            }
            _ => {}
        }
    }

    pub async fn process_data_packet(&self, packet: Packet) {
        let tx = self.inbound_data_stream_sender.lock().await.clone();
        if let Some(tx) = tx {
            if let Err(e) = tx.unbounded_send(packet) {
                log::error!("Failed to send data to peer {e:?}");
            }
        } else {
            log::warn!("No inbound data stream sender");
        }
    }

    pub async fn process_thumbnail_packet(&self, packet: Packet) {
        let tx = self.inbound_thumbnail_stream_sender.lock().await.clone();
        if let Some(tx) = tx {
            let _ = tx.unbounded_send(packet);
        }
    }

    pub async fn peer_disconnected(&self) {
        log::info!("Peer disconnected, handling canceling all transfers");
        self.inbound_thumbnail_stream_sender.lock().await.take();
        self.inbound_data_stream_sender.lock().await.take();
        self.transfers_context.stop_all().await;
        let response = CoreOperationOutput::P2P(P2POperationOutput::PeerDisconnected {});
        let _ = self.core_bridge.response(self.core_id.load(Ordering::Relaxed), response).await;
        self.core_id.store(0, Ordering::Relaxed);
    }

    pub async fn cancel_transfer(&self, session_id: u64) {
        let cancel_msg = CancelTransferSessionRequest {
            session_id: session_id as i64
        };

        self.transfers_context.stop_transfer(session_id).await;

        log::info!("Cancelling transfer session {session_id} to peer {}", self.peer.peer_id());
        let request = Request::CancelRequest(cancel_msg);
        let _ = self.msg_channel.notify(request).await;
    }

    pub async fn answer_transfer(
        &self,
        core_request_id: u32,
        session_id: u64,
        session: Option<TransferSession>
    ) -> Result<TransferSessionStatus, WebRtcErrors> {
        let (dat_tx, mut data_rx) = mpsc::unbounded();
        let (th_tx, mut thumbnail_rx) = mpsc::unbounded();

        self.inbound_data_stream_sender.lock().await.replace(dat_tx);
        self.inbound_thumbnail_stream_sender.lock().await.replace(th_tx);

        if session.is_none() {
            // Denied
            if let Some(rtc_request_id) = self.transfers_context.rtc_request_id(session_id).await {
                let response = TransferResponseMessage {};
                self.msg_channel.send_response(rtc_request_id, Response::TransferResponse(response)).await?;
            };

            return Ok(TransferSessionStatus::Canceled);
        }

        let mut session = session.unwrap();

        log::info!("Downloading transfer session {session_id} to peer {}", self.peer.peer_id());
        let msg_channel = self.msg_channel.clone();
        let peer_id = session.peer().unwrap().peer_id();
        let context = self.transfers_context.clone();
        let response = TransferResponseMessage {};
        if let Some(rtc_request_id) = context.rtc_request_id(session_id).await {
            if let Err(e) = msg_channel.send_response(rtc_request_id, Response::TransferResponse(response)).await {
                log::error!("Failed to send response to peer {peer_id}: {e:?}");
                context.stop_transfer(session_id).await;
            }
        }

        // Thumbnails download
        let mut thumbnail_paths = session
            .resources
            .iter()
            .filter_map(|r| r.thumbnail_path.clone().map(|it| (r.order_id, it)))
            .collect::<Vec<_>>();
        let repo = self.resource_repo.clone();
        let context = self.transfers_context.clone();
        let bridge = self.core_bridge.clone();
        let thumbnail_handle = spawn(async move {
            if thumbnail_paths.is_empty() {
                log::info!("Session {session_id} has no thumbnails to download");
                return Ok(())
            }

            // First delimiter
            loop {
                if !context.is_active(session_id).await {
                    return Ok(())
                }

                if thumbnail_paths.is_empty() {
                    return Ok(())
                }

                let first_delimiter = thumbnail_rx.recv_default_timeout().await.unwrap_or_default();
                let first_delimiter = TransferDelimiterShema::from_bytes(&first_delimiter)?;
                if !first_delimiter.is_start {
                    return Err(WebRtcErrors::InvalidDelimiter("The first must is_start = true".to_string()));
                }

                let index = thumbnail_paths.iter().position(|it| it.0 == first_delimiter.resource_id);

                if index.is_none() {
                    return Err(WebRtcErrors::InvalidDelimiter(format!(
                        "The first delimiter is not match with any resource {first_delimiter:?}"
                    )));
                }

                let resource_path = thumbnail_paths.swap_remove(index.unwrap()).1;
                if !context.is_active(session_id).await {
                    return Ok(())
                }

                let mut writer = repo.write(resource_path.clone()).await?;

                // Then we will download
                log::info!("Downloading thumbnail {resource_path:?} for session {session_id}");
                while let Some(bytes) = thumbnail_rx.recv_default_timeout().await {
                    if !context.is_active(session_id).await {
                        return Ok(());
                    }

                    writer
                        .write(bytes.to_vec().into())
                        .await
                        .map_err(|it| WebRtcErrors::PersistentError(PersistenceError::IOError(format!("{it:?}"))))?;
                    if let Ok(it) = TransferDelimiterShema::from_bytes(&bytes) {
                        if !it.is_start {
                            break;
                        } else {
                            return Err(WebRtcErrors::InvalidDelimiter("The first must is_start = false".to_string()));
                        }
                    }
                }

                writer.end().await?;
                let thumbnail_full_filled = ThumbnailFullFilled {
                    session_id,
                    resource_id: first_delimiter.resource_id,
                    local_resource_path: resource_path
                };

                let _ = bridge.response(core_request_id, CoreOperationOutput::Transfer(thumbnail_full_filled)).await;
            }
        });

        while !session.is_completed() {
            if !self.transfers_context.is_active(session_id).await {
                session.cancel();
                break;
            }

            let first_delimiter = data_rx.recv_default_timeout().await.unwrap_or_default();
            let first_delimiter = TransferDelimiterShema::from_bytes(&first_delimiter)?;
            if !first_delimiter.is_start {
                return Err(WebRtcErrors::InvalidDelimiter("The first must is_start = true".to_string()));
            }

            let Some(resource_path) = session
                .resources
                .iter()
                .find(|it| it.order_id == first_delimiter.resource_id)
                .map(|it| it.path.clone())
            else {
                return Err(WebRtcErrors::InvalidDelimiter(format!(
                    "The first delimiter is not match with any resource {first_delimiter:?}"
                )));
            };

            log::info!("Downloading resource {resource_path:?}");
            let mut writer = self.resource_repo.write(resource_path.clone()).await?;

            let progress_update = session.resource_mut_progress(first_delimiter.resource_id).unwrap();
            let mut total_written_bytes = 0u64;
            while let Some(packet) = data_rx.recv_default_timeout().await {
                if !self.transfers_context.is_active(session_id).await {
                    progress_update.fail("The session is canceled".to_string());
                    break;
                }

                if let Ok(end_delimiter) = TransferDelimiterShema::from_bytes(&packet) {
                    if end_delimiter.is_start {
                        return Err(WebRtcErrors::InvalidDelimiter("The end must is_start = false".to_string()));
                    }

                    progress_update.success();
                    log::info!("Received end delimiter");
                    break;
                }

                let written_bytes = packet.len() as u64;
                writer
                    .write(packet.to_vec().into())
                    .await
                    .map_err(|it| WebRtcErrors::PersistentError(PersistenceError::IOError(format!("{it:?}"))))?;
                total_written_bytes += written_bytes;
                progress_update.update_progress(written_bytes);
                self.core_bridge.resource_progress_update(core_request_id, progress_update, false).await;
            }

            writer.end().await?;
            log::info!("Downloaded resource {resource_path:?} len {total_written_bytes}");

            self.core_bridge.resource_progress_update(core_request_id, progress_update, true).await;
        }

        if let Err(err) = thumbnail_handle.await.unwrap() {
            log::error!("Failed to download thumbnails for session {session_id}: {err:?}");
        }

        self.transfers_context.stop_transfer(session_id).await;

        log::info!("Transfer session {session_id} completed");
        Ok(session.status())
    }

    pub async fn transfer_session(
        &self,
        core_request_id: u32,
        mut session: TransferSession
    ) -> Result<TransferSessionStatus, WebRtcErrors> {
        let request_id = uuid::Uuid::now_v7();
        self.transfers_context.start_transfer(session.order_id, request_id.to_string()).await;

        let session_id = session.order_id;
        log::info!("Sending transfer session {session_id} to peer {}", self.peer.peer_id());

        for resource in session.resources.iter_mut() {
            // We transfer folder as .tar file
            if matches!(resource.r#type, ResourceType::Folder) {
                resource.name = format!("{}.tar", resource.name);
            }
        }

        let transfer_session_message = TransferSessionMessage {
            order_id: session.order_id,
            resources: session.resources.iter().map(|r| r.to_proto()).collect()
        };

        let peer_id = session.peer().unwrap().peer_id();
        log::info!(target: "peer", "Sending session to peer {peer_id:?}: {transfer_session_message:?}", );
        let request = Request::TransferRequest(TransferRequestMessage {
            session: transfer_session_message
        });

        let _ = self.msg_channel.send(request, Some(request_id)).await?;

        log::info!("Transferring resources to peer {peer_id:?} {:?}", session.is_completed());

        // Transfer the thumbnails
        let session_thumbnail_paths = session
            .resources
            .iter()
            .filter_map(|r| r.thumbnail_path.clone().map(|it| (r.order_id, it)))
            .collect::<Vec<_>>();
        let repo = self.resource_repo.clone();
        let thumbnail_channel = self.thumbnail_channel.clone();
        let context = self.transfers_context.clone();
        let buffer = self.buffer.clone();
        let thumbnail_handle = spawn(async move {
            for (id, thumbnail_path) in session_thumbnail_paths {
                log::info!("Transferring thumbnail {thumbnail_path:?} for session {session_id}");
                if !context.is_active(session_id).await {
                    break;
                }

                let Ok(mut reader) = repo.read(thumbnail_path.clone(), 63 * 1024).await else {
                    continue;
                };

                let begin_delimiter = TransferDelimiterShema::new(id, true).as_bytes()?;

                if let Err(e) = thumbnail_channel.lock().await.send((peer_id, begin_delimiter)).await {
                    log::error!("Failed to send delimiter to peer for thumbnail {peer_id:?}: {e:?}");
                    return Err(WebRtcErrors::PersistentError(PersistenceError::IOError(format!("{e:?}"))));
                }

                while let Ok(Some(bytes)) = reader.next().await {
                    let bytes = Packet::from(bytes.to_vec());
                    let _ = thumbnail_channel.lock().await.send((peer_id, bytes)).await;

                    if buffer.sum_buffered_amount().await > MAX_BUFFER_SIZE {
                        buffer.flush_all_timeout().await?;
                    }
                }

                let end_delimiter = TransferDelimiterShema::new(id, false).as_bytes()?;
                if let Err(e) = thumbnail_channel.lock().await.send((peer_id, end_delimiter)).await {
                    log::error!("Failed to send delimiter to peer for thumbnail {peer_id:?}: {e:?}");
                    return Err(WebRtcErrors::PersistentError(PersistenceError::IOError(format!("{e:?}"))));
                }

                buffer.flush_all_timeout().await?;
            }

            Ok(())
        });

        while !session.is_completed() {
            if !self.transfers_context.is_active(session_id).await {
                log::info!("Session is canceled");
                session.cancel();
                break;
            }

            let Some((resource_path, order_id, size)) =
                session.get_next_transfer_resource().map(|it| (it.path.clone(), it.order_id, it.size))
            else {
                break;
            };

            let mut reader = self.resource_repo.read(resource_path.clone(), 63 * 1024).await?;
            log::info!("Transferring resource {resource_path:?} size {size} bytes");

            let mut total_sent_bytes = 0u64;
            let progress_update = session.resource_mut_progress(order_id).unwrap();
            let delimiter = TransferDelimiterShema::start(order_id).as_bytes()?;
            log::info!("Sending delimiter to peer {peer_id:?} len {}", delimiter.len());
            if let Err(e) = self.data_channel.lock().await.send((peer_id, delimiter)).await {
                let msg = format!("Failed to send delimiter to peer {peer_id:?}: {e:?}");
                progress_update.fail(msg);
                self.core_bridge.resource_progress_update(core_request_id, progress_update, false).await;
                continue;
            }

            while let Some(bytes) = reader.next().await.map_err(|e| WebRtcErrors::ReadFileError(format!("{e:?}")))? {
                if !self.transfers_context.is_active(session_id).await {
                    break;
                }

                let bytes = Packet::from(bytes.to_vec());
                let sent_bytes = bytes.len() as u64;
                total_sent_bytes += sent_bytes;
                let packet = (peer_id, bytes);
                let _ = self.data_channel.lock().await.send(packet).await;
                progress_update.update_progress(sent_bytes);
                self.core_bridge.resource_progress_update(core_request_id, progress_update, false).await;
                if self.buffer.sum_buffered_amount().await > MAX_BUFFER_SIZE {
                    self.buffer.flush_all_timeout().await?;
                }
            }

            let end_delimiter = TransferDelimiterShema::end(order_id).as_bytes()?;
            if let Err(e) = self.data_channel.lock().await.send((peer_id, end_delimiter)).await {
                let msg = format!("Failed to send delimiter to peer {peer_id:?}: {e:?}");
                progress_update.fail(msg);
                self.core_bridge.resource_progress_update(core_request_id, progress_update, false).await;
                continue;
            }

            log::info!(
                "Transfer resource {resource_path:?} completed with status {:?} total_sent {:?}",
                progress_update.status,
                total_sent_bytes
            );

            progress_update.complete();
            self.core_bridge.resource_progress_update(core_request_id, progress_update, false).await;
        }

        self.buffer.flush_all_timeout().await?;
        self.transfers_context.stop_transfer(session_id).await;

        if let Err(e) = thumbnail_handle.await.unwrap() {
            log::error!("Failed to transfer thumbnails for session {session_id}: {e:?}");
        }

        log::info!("Transfer session {session_id} completed");

        Ok(session.status())
    }

    pub async fn cancel_transfer_session(&self, session_id: u64) -> Result<(), WebRtcErrors> {
        self.transfers_context.stop_transfer(session_id).await;
        Ok(())
    }
}
