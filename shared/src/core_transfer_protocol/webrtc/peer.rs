use crate::app::file_system::file::{LocalResourcePath, ResourceType};
use crate::app::modules::transfer::TransferEvent::SessionResourceThumbnailFullFilled;
use crate::app::operations::p2p::P2POperationOutput;
use crate::app::operations::CoreOperationOutput;
use crate::app::repository::errors::PersistenceError;
use crate::app::repository::local_resource::LocalResourceRepository;
use crate::app::transfer::session::{TransferSession, TransferSessionStatus};
use crate::app::AppEvent;
use crate::core_api::{BufferExt, CoreBridge};
use crate::core_transfer_protocol::webrtc::errors::WebRtcErrors;
use crate::core_transfer_protocol::webrtc::message_channel::DirectMessageChannel;
use crate::core_transfer_protocol::webrtc::transfer::{TransferDelimiterShema, TransfersContext};
use crate::core_transfer_protocol::webrtc::webrtc::MAX_BUFFER_SIZE;
use crate::entities::peer::Peer as PeerEntity;
use core_services::utils::cancellation::{AbortError, AbortableExt};
use futures::channel::mpsc;
use futures::lock::Mutex;
use futures::StreamExt;
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
    pub peer: PeerEntity,
    pub core_bridge: Arc<dyn CoreBridge>,
    pub resource_repo: Arc<dyn LocalResourceRepository>,

    // Channel used to communicate with the peer
    pub msg_channel: DirectMessageChannel,
    // Channel used to transfer the resource
    pub data_channel: Arc<Mutex<mpsc::UnboundedSender<(PeerId, Packet)>>>,
    // This channel is used to transfer the thumbnail
    pub thumbnail_channel: Arc<Mutex<mpsc::UnboundedSender<(PeerId, Packet)>>>,
    // Webrtc buffer, used to control the amount of data that can be sent to the peer
    pub buffer: PeerBuffered,

    pub transfers_context: TransfersContext,

    pub inbound_thumbnail_stream_sender: Mutex<Option<mpsc::Sender<Packet>>>,
    pub inbound_data_stream_sender: Mutex<Option<mpsc::Sender<Packet>>>,

    // Connect to the core stream, where all state is stored
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

        let IntroduceResponse(response) = msg_channel.send(Request::IntroduceRequest(introduce_request), None).await? else {
            return Err(WebRtcErrors::FailedToIntroducePeer)
        };

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
    }

    pub async fn process_request(&self, request_id: String, msg: Request) {
        match msg {
            Request::CancelRequest(request) => {
                self.transfers_context.stop_transfer(request.session_id as u64).await;
            }
            Request::TransferRequest(request) => {
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
        let mut tx_opt = self.inbound_data_stream_sender.lock().await.clone();
        if let Some(tx) = tx_opt.as_mut() {
            if let Err(err) = tx.try_send(packet) {
                log::error!("Failed to send resource to peer {err:?}");
                tx_opt.take();
            }
        }
    }

    pub async fn process_thumbnail_packet(&self, packet: Packet) {
        let mut tx_opt = self.inbound_thumbnail_stream_sender.lock().await.clone();
        if let Some(tx) = tx_opt.as_mut() {
            if let Err(err) = tx.try_send(packet) {
                log::error!("Failed to send thumbnail to peer {err:?}");
                tx_opt.take();
            }
        }
    }

    pub async fn peer_disconnected(&self) {
        log::info!("Peer disconnected, will cancel all transfers");
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
        let Some(mut session) = session else {
            // Denied
            if let Some(rtc_request_id) = self.transfers_context.rtc_request_id(session_id).await {
                let response = TransferResponseMessage {};
                self.msg_channel.send_response(rtc_request_id, Response::TransferResponse(response)).await?;
            };

            return Ok(TransferSessionStatus::Canceled);
        };

        let Some(cancellation_signal) = self.transfers_context.signal(session.order_id).await else {
            return Err(WebRtcErrors::Canceled(AbortError::Cancelled))
        };

        let (resource_tx, mut resource_rx) = mpsc::channel(1024);
        let (thumbnail_tx, mut thumbnail_rx) = mpsc::channel(1024);

        self.inbound_data_stream_sender.lock().await.replace(resource_tx);
        self.inbound_thumbnail_stream_sender.lock().await.replace(thumbnail_tx);

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

        let mut thumbnail_paths = session
            .resources
            .iter()
            .filter_map(|r| r.thumbnail_path.clone().map(|it| (r.order_id, it)))
            .collect::<Vec<(u64, LocalResourcePath)>>();
        let repo = self.resource_repo.clone();
        let context = self.transfers_context.clone();
        let bridge = self.core_bridge.clone();

        let thumbnail_cancel_signal = cancellation_signal.clone();
        log::info!("Begin downloading thumbnails for session {session_id}");
        let thumbnail_handle = spawn(async move {
            while context.is_active(session_id).await {
                if thumbnail_paths.is_empty() {
                    return Ok(thumbnail_paths)
                }

                let first_delimiter = thumbnail_rx.next().abort_with(thumbnail_cancel_signal.clone()).await?.unwrap_or_default();
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
                    return Ok(thumbnail_paths)
                }

                let mut writer = repo.write(resource_path.clone()).await?;

                // Then we will download
                log::info!("Begin downloading thumbnail {resource_path:?} for session {session_id}");
                while let Ok(Some(bytes)) = thumbnail_rx.next().abort_with(thumbnail_cancel_signal.clone()).await {
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

                writer.end().abort_with(thumbnail_cancel_signal.clone()).await??;
                log::info!("Completed downloading thumbnail {resource_path:?}");

                let thumbnail_full_filled = SessionResourceThumbnailFullFilled {
                    session_id,
                    resource_id: first_delimiter.resource_id,
                    path: resource_path
                };

                let _ = bridge.notify(AppEvent::Transfer(thumbnail_full_filled)).await;
            }

            Ok(thumbnail_paths)
        });

        let resource_cancel_signal = cancellation_signal.clone();
        log::info!(
            "Begin downloading resources for session {session_id} {}",
            session.is_completed()
        );
        while !session.is_completed() {
            if !self.transfers_context.is_active(session_id).await {
                session.cancel();
                break;
            }

            let first_delimiter = resource_rx
                .next()
                .abort_with(resource_cancel_signal.clone())
                .await
                .unwrap_or_default()
                .unwrap_or_default();
            let first_delimiter = TransferDelimiterShema::from_bytes(&first_delimiter)?;
            if !first_delimiter.is_start {
                return Err(WebRtcErrors::InvalidDelimiter("The first must is_start = true".to_string()));
            }

            let Some((resource_path, resource_size)) = session
                .resources
                .iter()
                .find(|it| it.order_id == first_delimiter.resource_id)
                .map(|it| (it.path.clone(), it.size))
            else {
                return Err(WebRtcErrors::InvalidDelimiter(format!(
                    "The first delimiter is not match with any resource {first_delimiter:?}"
                )));
            };

            log::info!("Begin downloading resource {:?} {}", resource_path, resource_size);
            let mut writer = self.resource_repo.write(resource_path.clone()).await?;

            let progress_update = session.resource_mut_progress(first_delimiter.resource_id).unwrap();
            let mut total_written_bytes = 0u64;
            while let Ok(Some(packet)) = resource_rx.next().abort_with(resource_cancel_signal.clone()).await {
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
                    .abort_with(resource_cancel_signal.clone())
                    .await?
                    .map_err(|it| WebRtcErrors::PersistentError(PersistenceError::IOError(format!("{it:?}"))))?;
                total_written_bytes += written_bytes;
                progress_update.update_progress(written_bytes);
                self.core_bridge.resource_progress_update(core_request_id, progress_update, false).await;
            }

            writer.end().await?;
            progress_update.complete();
            self.core_bridge.resource_progress_update(core_request_id, progress_update, true).await;
            log::info!("Complete Downloading resource {:?} len {total_written_bytes}", resource_path);
        }

        let _ = thumbnail_handle.await;

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
        let Some(cancellation_signal) = self.transfers_context.signal(session.order_id).await else {
            return Err(WebRtcErrors::Canceled(AbortError::Cancelled))
        };

        let session_id = session.order_id;

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
        let request = Request::TransferRequest(TransferRequestMessage {
            session: transfer_session_message
        });

        let _ = self.msg_channel.send(request, Some(request_id)).await?;

        let mut session_thumbnail_paths = session
            .resources
            .iter()
            .filter_map(|r| r.thumbnail_path.clone().map(|it| (r.order_id, it)))
            .collect::<Vec<_>>();
        let repo = self.resource_repo.clone();
        let thumbnail_channel = self.thumbnail_channel.clone();
        let context = self.transfers_context.clone();
        let buffer = self.buffer.clone();
        let thumbnail_cancel_signal = cancellation_signal.clone();
        let thumbnail_handle = spawn(async move {
            while let Some((id, thumbnail_path)) = session_thumbnail_paths.pop() {
                if !context.is_active(session_id).await {
                    break;
                }

                log::info!("Begin transferring thumbnail {thumbnail_path:?} for session {session_id}");
                let Ok(Ok(mut reader)) = repo.read(thumbnail_path.clone(), 63 * 1024).abort_with(&thumbnail_cancel_signal).await
                else {
                    continue;
                };

                let begin_delimiter = TransferDelimiterShema::new(id, true).as_bytes()?;

                if let Err(e) = thumbnail_channel.lock().await.send((peer_id, begin_delimiter)).await {
                    log::error!("Failed to send delimiter to peer for thumbnail {peer_id:?}: {e:?}");
                    return Err(WebRtcErrors::PersistentError(PersistenceError::IOError(format!("{e:?}"))));
                }

                while let Ok(Ok(Some(bytes))) = reader.next().abort_with(&thumbnail_cancel_signal).await {
                    let bytes = Packet::from(bytes.to_vec());
                    if !bytes.is_empty() {
                        let _ = thumbnail_channel.lock().await.send((peer_id, bytes)).await;
                    }

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
                log::info!("Complete transferring thumbnail {thumbnail_path:?} for session {session_id}");
            }

            Ok(session_thumbnail_paths)
        });

        let resource_cancel_signal = cancellation_signal.clone();
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

            let mut reader = self
                .resource_repo
                .read(resource_path.clone(), 63 * 1024)
                .abort_with(resource_cancel_signal.clone())
                .await??;

            log::info!("Begin transferring resource {resource_path:?} size {size} bytes");
            let mut total_sent_bytes = 0u64;
            let progress_update = session.resource_mut_progress(order_id).unwrap();
            let delimiter = TransferDelimiterShema::start(order_id).as_bytes()?;
            if let Err(e) = self.data_channel.lock().await.send((peer_id, delimiter)).await {
                let msg = format!("Failed to send delimiter to peer {peer_id:?}: {e:?}");
                progress_update.fail(msg);
                self.core_bridge.resource_progress_update(core_request_id, progress_update, false).await;
                continue;
            }

            while let Some(bytes) = reader
                .next()
                .abort_with(&resource_cancel_signal)
                .await?
                .map_err(|e| WebRtcErrors::ReadFileError(format!("{e:?}")))?
            {
                let bytes = Packet::from(&bytes[..]);
                let sent_bytes = bytes.len() as u64;
                total_sent_bytes += sent_bytes;
                if !bytes.is_empty() {
                    let packet = (peer_id, bytes);
                    let _ = self.data_channel.lock().await.send(packet).await;
                }

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

            progress_update.complete();
            self.core_bridge.resource_progress_update(core_request_id, progress_update, false).await;

            log::info!(
                "Complete transferring resource {resource_path:?} with status {:?} total_sent {:?}",
                progress_update.status,
                total_sent_bytes
            );
        }

        self.buffer.flush_all_timeout().await?;
        let _ = thumbnail_handle.await;
        self.transfers_context.stop_transfer(session_id).await;
        log::info!("Transfer session {session_id} completed");

        Ok(session.status())
    }

    pub async fn cancel_transfer_session(&self, session_id: u64) -> Result<(), WebRtcErrors> {
        self.transfers_context.stop_transfer(session_id).await;
        Ok(())
    }
}
