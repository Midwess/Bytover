use std::collections::HashMap;
use n0_future::task::spawn;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use bytes::Buf;
use futures::channel::oneshot;
use futures::channel::mpsc;
use futures_util::lock::Mutex;
use futures_util::SinkExt;
use matchbox_protocol::{PeerId, PeerRequest};
use matchbox_socket::Packet;
use n0_future::StreamExt;
use serde::Serialize;
use n0_future::task::JoinHandle;
use core_services::db::repository::abstraction::table::Table;
use crate::entities::peer::Peer as PeerEntity;
use schema::devlog::bitbridge::{CancelTransferSessionRequest, IntroduceRequestMessage, IntroduceResponseMessage, PeerMessage, TransferRequestMessage, TransferResponseMessage, TransferSessionMessage};
use schema::devlog::bitbridge::peer_message_body::{Request, Response};
use schema::devlog::bitbridge::peer_message_body::Response::IntroduceResponse;
use crate::app::file_system::file::LocalResourcePath;
use crate::app::modules::transfer::TransferEvent::TransferRequest;
use crate::app::operations::CoreOperationOutput;
use crate::app::operations::p2p::P2POperationOutput;
use crate::app::operations::transfer::TransferOperationOutput;
use crate::app::repository::errors::PersistenceError;
use crate::app::repository::local_resource::LocalResourceRepository;
use crate::app::transfer::session::{TransferProgress, TransferSession, TransferSessionStatus};
use crate::core_api::{CoreBridge, IOReader, IOWriter};
use crate::core_transfer_protocol::webrtc::errors::WebRtcErrors;
use crate::core_transfer_protocol::webrtc::message_channel::DirectMessageChannel;
use crate::core_transfer_protocol::webrtc::transfer::{TransferDelimiterShema, TransfersContext};

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

    pub core_id: AtomicU32,
}

impl WebRtcPeer {
    pub async fn new(
        user: PeerEntity,
        msg_channel: DirectMessageChannel,
        core_bridge: Arc<dyn CoreBridge>,
        data_channel: mpsc::UnboundedSender<(PeerId, Packet)>,
        thumbnail_channel: mpsc::UnboundedSender<(PeerId, Packet)>,
        repository: Arc<dyn LocalResourceRepository>,
    ) -> Result<Self, WebRtcErrors> {
        let introduce_request = IntroduceRequestMessage {
            mine: PeerMessage {
                peer_id: user.id().to_string(),
                name: user.name.clone(),
                avatar_url: user.avatar_url.clone(),
                device: user.device.clone().into(),
                email: user.email.clone(),
            }
        };

        let IntroduceResponse(response) = msg_channel
            .send(Request::IntroduceRequest(introduce_request), None)
            .await? else {
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
            core_id: Default::default(),
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
        repository: Arc<dyn LocalResourceRepository>,
    ) -> Result<Self, WebRtcErrors> {
        let introduce_response = IntroduceResponse(IntroduceResponseMessage {
            peer: PeerMessage {
                peer_id: user.id().to_string(),
                name: user.name.clone(),
                avatar_url: user.avatar_url.clone(),
                device: user.device.clone().into(),
                email: user.email.clone(),
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
            core_id: Default::default(),
        })
    }

    pub fn start_core_stream(&self, core_stream_id: u32) {
        self.core_id.store(core_stream_id, Ordering::Relaxed)
    }

    pub async fn process_request(&self, msg: Request) {
       match msg {
           Request::CancelRequest(request) => {
               let response = CoreOperationOutput::P2P(P2POperationOutput::CancelSessionRequest {
                   session_id: request.session_id as u64
               });

               let _ = self.core_bridge.response(self.core_id.load(Ordering::Relaxed), response).await;
           }
           Request::TransferRequest(request) => {
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
            let _ = tx.unbounded_send(packet);
        }
    }

    pub async fn peer_disconnected(&self) {
        self.transfers_context.stop_all().await;
        self.inbound_thumbnail_stream_sender.lock().await.take();
        self.inbound_data_stream_sender.lock().await.take();
        let response = CoreOperationOutput::P2P(P2POperationOutput::PeerDisconnected {});
        let _ = self.core_bridge.response(self.core_id.load(Ordering::Relaxed), response).await;
        self.core_id.store(0, Ordering::Relaxed);
    }

    pub async fn cancel_transfer(&self, session_id: u64) {
        let cancel_msg = CancelTransferSessionRequest {
            session_id: session_id as i64
        };

        self.transfers_context.stop_transfer(session_id).await;

        let request = Request::CancelRequest(cancel_msg);
        let _ = self.msg_channel.notify(request);
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
        let start_handle = spawn(async move {
            let response = TransferResponseMessage {};
            if let Some(rtc_request_id) = context.rtc_request_id(session_id).await {
                if let Err(e) = msg_channel.send_response(rtc_request_id, Response::TransferResponse(response)).await {
                    log::error!("Failed to send response to peer {peer_id}: {e:?}");
                    context.stop_transfer(session_id).await;
                }
            }
        });

        // Thumbnails download
        let mut thumbnail_paths = session.resources.iter().filter_map(|r| r.thumbnail_path.clone().map(|it| (r.order_id, it))).collect::<Vec<_>>();
        let repo = self.resource_repo.clone();
        let context = self.transfers_context.clone();
        let thumbnail_handle = spawn(async move {
            // First delimiter
            if thumbnail_paths.is_empty() {
                return Ok(())
            }

            let first_delimiter = thumbnail_rx.next().await.unwrap_or_default();
            let first_delimiter = TransferDelimiterShema::from_bytes(&first_delimiter)?;
            if !first_delimiter.is_start {
                return Err(WebRtcErrors::InvalidDelimiter("The first must is_start = true".to_string()));
            }

            first_delimiter.resource_id;
            let index = thumbnail_paths
                .iter()
                .position(|it| it.0 == first_delimiter.resource_id);

            if index.is_none() {
                return Err(WebRtcErrors::InvalidDelimiter(format!("The first delimiter is not match with any resource {first_delimiter:?}")));
            }

            let resource_path = thumbnail_paths.swap_remove(index.unwrap()).1;
            if !context.is_active(session_id).await {
                return Ok(())
            }

            let mut writer = repo.write(resource_path).await?;

            // Then we will download
            while let Some(bytes) = thumbnail_rx.next().await {
                if !context.is_active(session_id).await {
                    return Ok(());
                }

                writer.write(bytes.to_vec().into()).await.map_err(|it| WebRtcErrors::PersistentError(PersistenceError::IOError(format!("{it:?}"))))?;
                if let Ok(it) = TransferDelimiterShema::from_bytes(&bytes) {
                    if !it.is_start {
                        break;
                    }
                    else {
                        return Err(WebRtcErrors::InvalidDelimiter("The first must is_start = false".to_string()));
                    }
                }
            }

            Ok(())
        });

        while !session.is_completed() {
            if !self.transfers_context.is_active(session_id).await {
                session.cancel();
                break;
            }

            let first_delimiter = data_rx.next().await.unwrap_or_default();
            let first_delimiter = TransferDelimiterShema::from_bytes(&first_delimiter)?;
            if !first_delimiter.is_start {
                return Err(WebRtcErrors::InvalidDelimiter("The first must is_start = true".to_string()));
            }

            let Some(resource_path) = session
                .resources
                .iter()
                .find(|it| it.order_id == first_delimiter.resource_id).map(|it| it.path.clone()) else {
                return Err(WebRtcErrors::InvalidDelimiter(format!("The first delimiter is not match with any resource {first_delimiter:?}")));
            };

            let mut writer = self.resource_repo.write(resource_path.clone()).await?;

            let progress_update = session.resource_mut_progress(first_delimiter.resource_id).unwrap();
            let mut total_written_bytes = 0u64;
            for packet in data_rx.next().await {
                if !self.transfers_context.is_active(session_id).await {
                    progress_update.fail("The session is canceled".to_string());
                    break;
                }

                if let Ok(end_delimiter) = TransferDelimiterShema::from_bytes(&packet) {
                    if end_delimiter.is_start {
                        return Err(WebRtcErrors::InvalidDelimiter("The first must is_start = false".to_string()));
                    }

                    progress_update.success();
                    break;
                }

                let written_bytes = packet.len() as u64;
                writer.write(packet.to_vec().into()).await.map_err(|it| WebRtcErrors::PersistentError(PersistenceError::IOError(format!("{it:?}"))))?;
                total_written_bytes += written_bytes;
                progress_update.update_progress(written_bytes);
                self.core_bridge.resource_progress_update(core_request_id, progress_update).await;
            }

            self.core_bridge.resource_progress_update(core_request_id, progress_update).await;
        }

        let _ = start_handle.await.unwrap();
        let _ = thumbnail_handle.await.unwrap()?;

        Ok(session.status())
    }

    pub async fn transfer_session(
        &self,
        core_request_id: u32,
        mut session: TransferSession,
    ) -> Result<TransferSessionStatus, WebRtcErrors> {
        let request_id = uuid::Uuid::new_v4();
        self.transfers_context.start_transfer(session.order_id, request_id.to_string()).await;

        let session_id = session.order_id;
        log::info!("Sending transfer session {session_id} to peer {}", self.peer.peer_id());

        let transfer_session_message = TransferSessionMessage {
            order_id: session.order_id,
            resources: session.resources.iter().map(|r| r.to_proto()).collect()
        };

        let peer_id = session.peer().unwrap().peer_id();
        log::info!(target: "peer", "Sending session to peer {peer_id:?}", );
        let request = Request::TransferRequest(TransferRequestMessage {
            session: transfer_session_message
        });

        let _ = self.msg_channel.send(request, Some(request_id)).await?;

        log::info!("Transferring resources to peer {peer_id:?}");

        // Transfer the thumbnails
        let session_thumbnail_paths = session.resources.iter().filter_map(|r| r.thumbnail_path.clone()).collect::<Vec<_>>();
        let repo = self.resource_repo.clone();
        let thumbnail_channel = self.thumbnail_channel.clone();
        let context = self.transfers_context.clone();
        let thumbnail_handle = spawn(async move {
            for thumbnail_path in session_thumbnail_paths {
                if !context.is_active(session_id).await {
                    break;
                }

                let Ok(mut reader) = repo.read(thumbnail_path.clone(), 63 * 1024).await else {
                    continue;
                };

                let begin_delimiter = TransferDelimiterShema::new(session.order_id, true)
                    .as_bytes()?;

                if let Err(e) = thumbnail_channel.lock().await.send((peer_id.clone(), begin_delimiter)).await {
                    log::error!("Failed to send delimiter to peer for thumbnail {peer_id:?}: {e:?}");
                    return Err(WebRtcErrors::PersistentError(PersistenceError::IOError(format!("{e:?}"))));
                }

                while let Ok(Some(bytes)) = reader.next().await {
                    let bytes = Packet::from(bytes.to_vec());
                    let _ = thumbnail_channel.lock().await.send((peer_id.clone(), bytes)).await;
                }

                let end_delimiter = TransferDelimiterShema::new(session.order_id, false).as_bytes()?;
                if let Err(e) = thumbnail_channel.lock().await.send((peer_id.clone(), end_delimiter)).await {
                    log::error!("Failed to send delimiter to peer for thumbnail {peer_id:?}: {e:?}");
                    return Err(WebRtcErrors::PersistentError(PersistenceError::IOError(format!("{e:?}"))));
                }
            }

            Ok(())
        });

        while !session.is_completed() {
            if !self.transfers_context.is_active(session_id).await {
                session.cancel();
                break;
            }

            let Some((resource_path, order_id)) = session.get_next_transfer_resource()
                .map(|it| (it.path.clone(), it.order_id)) else {
                break;
            };

            let mut reader = self.resource_repo.read(resource_path.clone(), 63 * 1024).await?;
            log::info!("Transferring resource {resource_path:?}" );

            let mut total_sent_bytes = 0u64;
            let progress_update = session.resource_mut_progress(order_id).unwrap();
            let delimiter = TransferDelimiterShema::new(order_id, true).as_bytes()?;
            if let Err(e) = self.data_channel.lock().await.send((peer_id.clone(), delimiter)).await {
                let msg = format!("Failed to send delimiter to peer {peer_id:?}: {e:?}");
                progress_update.fail(msg);
                self.core_bridge.resource_progress_update(core_request_id, progress_update).await;
                continue;
            }

            while let Some(bytes) = reader.next().await.map_err(|e| WebRtcErrors::ReadFileError(format!("{e:?}")))? {
                if !self.transfers_context.is_active(session_id).await {
                    break;
                }

                let bytes = Packet::from(bytes.to_vec());
                let sent_bytes = bytes.len() as u64;
                total_sent_bytes += sent_bytes;
                let packet = (peer_id.clone(), bytes);
                let _ = self.data_channel.lock().await.send(packet).await;
                progress_update.update_progress(sent_bytes);
                self.core_bridge.resource_progress_update(core_request_id, progress_update).await;
            }

            let end_delimiter = TransferDelimiterShema::new(order_id, false).as_bytes()?;
            if let Err(e) = self.data_channel.lock().await.send((peer_id.clone(), end_delimiter)).await {
                let msg = format!("Failed to send delimiter to peer {peer_id:?}: {e:?}");
                progress_update.fail(msg);
                self.core_bridge.resource_progress_update(core_request_id, progress_update).await;
                continue;
            }

            log::info!("Transfer resource {resource_path:?} completed with status {:?}", progress_update.status);
            progress_update.complete();
            self.core_bridge.resource_progress_update(core_request_id, progress_update).await;
        }

        let _ = thumbnail_handle.await;

        Ok(session.status())
    }

    pub fn cancel_transfer_session(&self) -> Result<(), WebRtcErrors> {
        Ok(())
    }
}
