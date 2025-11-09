use crate::app::operations::p2p::P2POperationOutput;
use crate::app::operations::transfer::TransferOperationOutput;
use crate::app::operations::transfer::TransferOperationOutput::TransferResourceProgressUpdate;
use crate::app::operations::CoreOperationOutput;
use crate::entities::local_resource::{LocalResourcePath, ResourceType};
use crate::entities::peer::Peer as PeerEntity;
use crate::entities::transfer_session::{ThumbnailUpdatedEvent, TransferSession, TransferSessionStatus};
use crate::protocol::webrtc::errors::WebRtcErrors;
use crate::protocol::webrtc::message_channel::DirectMessageChannel;
use crate::protocol::webrtc::transfer::{TransferDelimiterShema, TransfersContext};
use crate::protocol::webrtc::webrtc::{MAX_BUFFER_SIZE, TRANSFER_RESOURCE_CHANNEL_ID, TRANSFER_THUMBNAIL_CHANNEL_ID};
use crate::repository::errors::PersistenceError;
use crate::repository::local_resource::LocalResourceRepository;
use crate::shell::api::{BufferExt, CoreRequest};
use core_services::utils::cancellation::{FutureExtension, TaskErrors};
use core_services::utils::yield_container::YieldContainer;
use futures::channel::mpsc;
use futures::StreamExt;
use matchbox_protocol::PeerId;
use matchbox_socket::{Packet, PeerBuffered};
use n0_future::task::spawn;
use once_cell::sync::OnceCell;
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
use std::sync::Arc;
use std::time::Duration;

pub struct WebRtcPeer {
    pub peer: PeerEntity,
    pub resource_repo: Arc<dyn LocalResourceRepository>,

    // Channel used to communicate with the peer
    pub msg_channel: DirectMessageChannel,
    // Channel used to transfer the resource
    pub data_channel: mpsc::UnboundedSender<(PeerId, Packet)>,
    // This channel is used to transfer the thumbnail
    pub thumbnail_channel: mpsc::UnboundedSender<(PeerId, Packet)>,
    // Webrtc buffer, used to control the amount of data that can be sent to the peer
    pub buffer: PeerBuffered,

    pub transfers_context: TransfersContext,

    pub inbound_thumbnail_stream_receiver: YieldContainer<mpsc::Receiver<Packet>>,
    pub inbound_thumbnail_stream_sender: mpsc::Sender<Packet>,
    pub inbound_data_stream_receiver: YieldContainer<mpsc::Receiver<Packet>>,
    pub inbound_data_stream_sender: mpsc::Sender<Packet>,

    // Connect to the core stream, where all state is stored
    pub core_request: OnceCell<CoreRequest>
}

impl WebRtcPeer {
    pub async fn new(
        user: PeerEntity,
        msg_channel: DirectMessageChannel,
        data_channel: mpsc::UnboundedSender<(PeerId, Packet)>,
        thumbnail_channel: mpsc::UnboundedSender<(PeerId, Packet)>,
        buffer: PeerBuffered,
        repository: Arc<dyn LocalResourceRepository>
    ) -> Result<Self, WebRtcErrors> {
        let (thumbnail_data_tx, thumbnail_data_rx) = mpsc::channel(1024);
        let (data_tx, data_rx) = mpsc::channel(1024);

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
            data_channel,
            thumbnail_channel,
            transfers_context: TransfersContext::new(),
            resource_repo: repository,
            inbound_thumbnail_stream_sender: thumbnail_data_tx,
            inbound_data_stream_sender: data_tx,
            inbound_data_stream_receiver: YieldContainer::new(data_rx),
            inbound_thumbnail_stream_receiver: YieldContainer::new(thumbnail_data_rx),
            buffer,
            core_request: Default::default()
        })
    }

    pub fn core_request(&self) -> &CoreRequest {
        self.core_request.get().expect("Core request is not set")
    }

    pub async fn from_introduce_request(
        user: PeerEntity,
        request_id: String,
        msg: IntroduceRequestMessage,
        msg_channel: DirectMessageChannel,
        data_channel: mpsc::UnboundedSender<(PeerId, Packet)>,
        thumbnail_channel: mpsc::UnboundedSender<(PeerId, Packet)>,
        buffer: PeerBuffered,
        repository: Arc<dyn LocalResourceRepository>
    ) -> Result<Self, WebRtcErrors> {
        log::info!("Received introduce request from other peer {:?}", msg.mine.peer_id);
        let (thumbnail_data_tx, thumbnail_data_rx) = mpsc::channel(1024);
        let (data_tx, data_rx) = mpsc::channel(1024);
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
            data_channel,
            thumbnail_channel,
            transfers_context: TransfersContext::new(),
            resource_repo: repository,
            inbound_thumbnail_stream_sender: thumbnail_data_tx,
            inbound_data_stream_sender: data_tx,
            inbound_data_stream_receiver: YieldContainer::new(data_rx),
            inbound_thumbnail_stream_receiver: YieldContainer::new(thumbnail_data_rx),
            buffer,
            core_request: Default::default()
        })
    }

    pub fn start_core_stream(&self, core_request: CoreRequest) {
        let _ = self.core_request.set(core_request);
    }

    pub async fn process_message_packet(&self, request_id: String, msg: Request) {
        match msg {
            Request::CancelRequest(request) => {
                self.transfers_context.stop_transfer(request.session_id as u64).await;
            }
            Request::TransferRequest(request) => {
                self.transfers_context.start_transfer(request.session.order_id, request_id).await;
                let response = CoreOperationOutput::P2P(P2POperationOutput::ReceivedSessionRequest {
                    remote_session: request.session
                });

                let _ = self.core_request().response(response).await;
            }
            _ => {}
        }
    }

    pub async fn process_data_packet(&self, packet: Packet) {
        let _ = self.inbound_data_stream_sender.clone().try_send(packet);
    }

    pub async fn process_thumbnail_packet(&self, packet: Packet) {
        let _ = self.inbound_thumbnail_stream_sender.clone().try_send(packet);
    }

    pub async fn peer_disconnected(&self) {
        log::info!("Peer disconnected, will cancel all transfers");
        self.transfers_context.stop_all().await;
        let response = CoreOperationOutput::P2P(P2POperationOutput::PeerDisconnected {});
        let _ = self.core_request().response(response).await;
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
        core_request: CoreRequest,
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

        let Some(cancellation_signal) = self.transfers_context.cancellation_token(session.order_id).await else {
            return Err(WebRtcErrors::Canceled(TaskErrors::Cancelled))
        };

        let _drop_guard = cancellation_signal.drop_guard();

        let mut resource_rx = self.inbound_data_stream_receiver.retrieve().await?;
        let mut thumbnail_rx = self.inbound_thumbnail_stream_receiver.retrieve().await?;

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

        let thumbnail_handle = {
            let mut thumbnail_paths = session
                .resources
                .iter()
                .filter_map(|r| r.thumbnail_path.clone().map(|it| (r.order_id, it)))
                .collect::<Vec<(u64, LocalResourcePath)>>();
            let repo = self.resource_repo.clone();
            let context = self.transfers_context.clone();
            let core_request = core_request.clone();
            let thumbnail_cancel_signal = cancellation_signal.child_token();
            spawn(async move {
                while context.is_active(session_id).await {
                    if thumbnail_paths.is_empty() {
                        return Ok(thumbnail_paths)
                    }

                    log::info!("Begin receiving thumbnail for session {session_id}");
                    let start_delimiter = TransferDelimiterShema::forward_to_next_resource(&mut thumbnail_rx, session_id)
                        .with_cancel(&thumbnail_cancel_signal)
                        .await??;

                    log::info!("Found start delimiter {start_delimiter:?}");

                    let Some(resource_index) = thumbnail_paths.iter().position(|it| it.0 == start_delimiter.resource_id) else {
                        return Err(WebRtcErrors::InvalidDelimiter(format!(
                            "The first delimiter is not match with any resource {start_delimiter:?}"
                        )));
                    };

                    let resource_path = thumbnail_paths.swap_remove(resource_index).1;
                    if !context.is_active(session_id).await {
                        return Ok(thumbnail_paths)
                    }

                    let mut writer = repo.write(resource_path.clone()).with_cancel(&thumbnail_cancel_signal).await??;

                    while let Ok(Some(bytes)) = thumbnail_rx.next().with_cancel(&thumbnail_cancel_signal).await {
                        writer
                            .write(bytes.to_vec().into())
                            .await
                            .map_err(|it| WebRtcErrors::PersistentError(PersistenceError::IOError(format!("{it:?}"))))?;

                        if TransferDelimiterShema::from_end_packet(&bytes, session_id).is_ok() {
                            break;
                        }
                    }

                    writer.end().with_cancel(&thumbnail_cancel_signal).await??;

                    let event = ThumbnailUpdatedEvent {
                        resource_id: start_delimiter.resource_id,
                        path: resource_path
                    };

                    let _ = core_request.response(TransferOperationOutput::ThumbnailUpdated(event)).await;
                }

                Ok(thumbnail_paths)
            })
        };

        while !session.is_completed() {
            if !self.transfers_context.is_active(session_id).await {
                session.cancel();
                break;
            }

            let start_delimiter = TransferDelimiterShema::forward_to_next_resource(&mut resource_rx, session_id)
                .with_cancel(&cancellation_signal)
                .await??;

            let Some((resource_path, resource_size)) = session
                .resources
                .iter()
                .find(|it| it.order_id == start_delimiter.resource_id)
                .map(|it| (it.path.clone(), it.size))
            else {
                return Err(WebRtcErrors::InvalidDelimiter(format!(
                    "The first delimiter is not match with any resource {start_delimiter:?}"
                )));
            };

            let mut writer = self.resource_repo.write(resource_path.clone()).await?;

            let progress_update = session.resource_mut_progress(start_delimiter.resource_id).unwrap();
            let mut total_written_bytes = 0u64;
            log::info!("Begin downloading resource {:?} {}", resource_path, resource_size);
            while let Ok(Some(packet)) = resource_rx.next().with_cancel(&cancellation_signal).await {
                if TransferDelimiterShema::from_end_packet(&packet, session_id).is_ok() {
                    progress_update.success();
                    break;
                }

                let written_bytes = packet.len() as u64;
                writer
                    .write(packet.to_vec().into())
                    .with_cancel(&cancellation_signal)
                    .await?
                    .map_err(|it| WebRtcErrors::PersistentError(PersistenceError::IOError(format!("{it:?}"))))?;
                total_written_bytes += written_bytes;
                progress_update.update_progress(written_bytes);
                core_request.response_throttle(TransferResourceProgressUpdate(progress_update.clone())).await;
            }

            log::info!("Complete downloading resource {:?} len {total_written_bytes}", resource_path);
            writer.end().await?;
            progress_update.complete();
            let _ = core_request.response(TransferResourceProgressUpdate(progress_update.clone())).await;
        }

        // Giving max 10s more for thumbnail to complete
        cancellation_signal.cancel_after(Duration::from_secs(10));
        let _ = thumbnail_handle.await;
        self.transfers_context.stop_transfer(session_id).await;

        Ok(session.status())
    }

    pub async fn transfer_session(
        &self,
        core_request: CoreRequest,
        mut session: TransferSession
    ) -> Result<TransferSessionStatus, WebRtcErrors> {
        let request_id = uuid::Uuid::now_v7();
        self.transfers_context.start_transfer(session.order_id, request_id.to_string()).await;
        let Some(cancellation_signal) = self.transfers_context.cancellation_token(session.order_id).await else {
            return Err(WebRtcErrors::Canceled(TaskErrors::Cancelled))
        };

        let _drop_guard = cancellation_signal.drop_guard();

        let session_id = session.order_id;
        log::info!("Requesting peer to transfer session {session_id}");

        for resource in session.resources.iter_mut() {
            if matches!(resource.r#type, ResourceType::Folder) {
                resource.name = format!("{}.zip", resource.name);
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

        let response = self.msg_channel.send(request, Some(request_id)).await?;
        log::info!("Received response for session {session_id} {response:?}");

        let mut session_thumbnail_paths = session
            .resources
            .iter()
            .filter_map(|r| r.thumbnail_path.clone().map(|it| (r.order_id, it)))
            .rev()
            .collect::<Vec<_>>();

        let repo = self.resource_repo.clone();
        let thumbnail_channel = self.thumbnail_channel.clone();
        let context = self.transfers_context.clone();
        let buffer = self.buffer.clone();
        let thumbnail_handle = {
            let thumbnail_cancel_signal = cancellation_signal.clone();
            spawn(async move {
                while let Some((id, thumbnail_path)) = session_thumbnail_paths.pop() {
                    if !context.is_active(session_id).await {
                        break;
                    }

                    let Ok(Ok(mut reader)) = repo.read(thumbnail_path.clone(), 63 * 1024).with_cancel(&thumbnail_cancel_signal).await
                    else {
                        continue;
                    };

                    let begin_delimiter = TransferDelimiterShema::new(session_id, id, true).as_bytes()?;

                    if let Err(e) = thumbnail_channel.unbounded_send((peer_id, begin_delimiter)) {
                        log::error!("Failed to send delimiter to peer for thumbnail {peer_id:?}: {e:?}");
                        return Err(WebRtcErrors::PersistentError(PersistenceError::IOError(format!("{e:?}"))));
                    }

                    while let Ok(Ok(Some(bytes))) = reader.next(None).with_cancel(&thumbnail_cancel_signal).await {
                        let bytes = Packet::from(bytes);
                        if !bytes.is_empty() {
                            let _ = thumbnail_channel.unbounded_send((peer_id, bytes));
                        }

                        if buffer.buffered_amount(TRANSFER_THUMBNAIL_CHANNEL_ID).await > MAX_BUFFER_SIZE {
                            buffer.flush_timeout(TRANSFER_THUMBNAIL_CHANNEL_ID).await?;
                        }
                    }

                    let end_delimiter = TransferDelimiterShema::new(session_id, id, false).as_bytes()?;
                    if let Err(e) = thumbnail_channel.unbounded_send((peer_id, end_delimiter)) {
                        log::error!("Failed to send delimiter to peer for thumbnail {peer_id:?}: {e:?}");
                        return Err(WebRtcErrors::PersistentError(PersistenceError::IOError(format!("{e:?}"))));
                    }
                }

                Ok(session_thumbnail_paths)
            })
        };

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
                .with_cancel(&resource_cancel_signal)
                .await??;

            log::info!("Begin transferring resource {resource_path:?} size {size} bytes");
            let mut total_sent_bytes = 0u64;
            let progress_update = session.resource_mut_progress(order_id).unwrap();
            let delimiter = TransferDelimiterShema::start(session_id, order_id).as_bytes()?;
            if let Err(e) = self.data_channel.unbounded_send((peer_id, delimiter)) {
                let msg = format!("Failed to send delimiter to peer {peer_id:?}: {e:?}");
                progress_update.fail(msg);
                let _ = core_request.response_throttle(TransferResourceProgressUpdate(progress_update.clone())).await;
                continue;
            }

            while let Some(bytes) = reader
                .next(None)
                .with_cancel(&resource_cancel_signal)
                .await?
                .map_err(|e| WebRtcErrors::ReadFileError(format!("{e:?}")))?
            {
                let bytes = Packet::from(bytes);
                let sent_bytes = bytes.len() as u64;
                total_sent_bytes += sent_bytes;
                if !bytes.is_empty() {
                    let packet = (peer_id, bytes);
                    let _ = self.data_channel.unbounded_send(packet);
                }

                progress_update.update_progress(sent_bytes);
                let _ = core_request.response_throttle(TransferResourceProgressUpdate(progress_update.clone())).await;
                if self.buffer.buffered_amount(TRANSFER_RESOURCE_CHANNEL_ID).await > MAX_BUFFER_SIZE {
                    self.buffer.flush_timeout(TRANSFER_RESOURCE_CHANNEL_ID).await?;
                }
            }

            let end_delimiter = TransferDelimiterShema::end(session_id, order_id).as_bytes()?;
            if let Err(e) = self.data_channel.unbounded_send((peer_id, end_delimiter)) {
                let msg = format!("Failed to send delimiter to peer {peer_id:?}: {e:?}");
                progress_update.fail(msg);
                let _ = core_request.response(TransferResourceProgressUpdate(progress_update.clone()));
                continue;
            }

            progress_update.complete();
            let _ = core_request.response(TransferResourceProgressUpdate(progress_update.clone())).await;

            log::info!(
                "Complete transferring resource {resource_path:?} with status {:?} total_sent {:?}",
                progress_update.status,
                total_sent_bytes
            );
        }

        // Giving max 10s more for thumbnail to complete
        cancellation_signal.cancel_after(Duration::from_secs(10));
        let _ = thumbnail_handle.await;
        self.buffer.flush_all_timeout().await?;
        self.transfers_context.stop_transfer(session_id).await;
        log::info!("Transfer session {session_id} completed");

        Ok(session.status())
    }

    pub async fn cancel_transfer_session(&self, session_id: u64) -> Result<(), WebRtcErrors> {
        self.transfers_context.stop_transfer(session_id).await;
        Ok(())
    }
}
