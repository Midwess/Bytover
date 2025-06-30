use std::collections::HashMap;
use std::sync::{Arc};
use futures::select;
use std::time::Duration;
use futures_timer::Delay;
use futures_util::FutureExt;
use futures_util::lock::Mutex;
use matchbox_protocol::PeerId;
use matchbox_socket::WebRtcSocket;
use ulid::Ulid;
use uuid::Uuid;
use n0_future::task::spawn;
use schema::devlog::bitbridge::PeerMessageBody;
use crate::core_api::CoreBridge;
use crate::core_transfer_protocol::webrtc::errors::WebRtcErrors;
use crate::core_transfer_protocol::webrtc::message_channel::DirectMessageChannel;
use crate::core_transfer_protocol::webrtc::peer::{WebRtcPeer};
use crate::core_transfer_protocol::webrtc::signalling::{WebSignaller, WebSignallerBuilder};
use crate::core_transfer_protocol::webrtc::signalling_client::SignallingClient;
use prost::Message;
use schema::devlog::bitbridge::peer_message_body::Request;
use crate::app::operations::CoreOperationOutput;
use crate::app::operations::p2p::P2POperationOutput;
use crate::app::repository::local_resource::LocalResourceRepository;
use crate::entities::peer::Peer as PeerEntity;

pub static MSG_CHANNEL_ID: usize = 0;
pub static TRANSFER_RESOURCE_CHANNEL_ID: usize = 1;
pub static TRANSFER_THUMBNAIL_CHANNEL_ID: usize = 1;

pub struct WebRtc {
    peers: Arc<Mutex<HashMap<PeerId, WebRtcPeer>>>,
    core_bridge: Arc<dyn CoreBridge>,
    addr: String,
    local_resource_repository: Arc<dyn LocalResourceRepository>,
}

impl WebRtc {
    fn new(core_bridge: Arc<dyn CoreBridge>, addr: String, local_resource_repository: Arc<dyn LocalResourceRepository>) -> Self {
        Self {
            peers: Default::default(),
            core_bridge,
            addr,
            local_resource_repository
        }
    }

    async fn start(&self, core_request_id: u32, current_user: PeerEntity) -> Result<(), WebRtcErrors> {
        let signaller_builder = Arc::new(WebSignallerBuilder::new());
        let (mut socket, loop_fut) = WebRtcSocket::builder(self.addr.clone())
            .signaller_builder(signaller_builder.clone())
            .add_reliable_channel()
            .add_reliable_channel()
            .add_reliable_channel()
            .signaling_keep_alive_interval(Some(Duration::from_secs(3)))
            .build();

        let loop_fut = loop_fut.fuse();
        futures::pin_mut!(loop_fut);
        let timeout = Delay::new(Duration::from_millis(100));
        futures::pin_mut!(timeout);

        let outbound_msg_sender = socket.channel(MSG_CHANNEL_ID).sender_clone();
        let outbound_data_sender = socket.channel(TRANSFER_RESOURCE_CHANNEL_ID).sender_clone();
        let outbound_thumbnail_sender = socket.channel(TRANSFER_THUMBNAIL_CHANNEL_ID).sender_clone();

        let mut handles = vec![];

        loop {
            for (peer_id, state) in socket.try_update_peers()? {
               if state == matchbox_socket::PeerState::Connected {
                   let peers_guard = self.peers.lock().await;
                   if peers_guard.contains_key(&peer_id) {
                       log::warn!("Skip the peer since it already exists");
                       continue;
                   }

                   if peer_id < current_user.peer_id() {
                       let direct_message_channel = DirectMessageChannel::new(peer_id.clone(), outbound_msg_sender.clone());
                       let core_bridge = self.core_bridge.clone();
                       let peers = self.peers.clone();
                       let current_user = current_user.clone();
                       let outbound_data_sender = outbound_data_sender.clone();
                       let outbound_thumbnail_sender = outbound_thumbnail_sender.clone();
                       let local_resource_repository = self.local_resource_repository.clone();
                       handles.push(spawn(async move {
                           let peer = match WebRtcPeer::new(
                               current_user.clone(),
                               direct_message_channel,
                               core_bridge.clone(),
                               outbound_data_sender,
                               outbound_thumbnail_sender,
                               local_resource_repository
                           ).await {
                               Ok(peer) => peer,
                               Err(err) => {
                                   log::error!("Failed to connect to peer {:?}", err);
                                   return;
                               }
                           };

                           let peer_entity = peer.peer.clone();
                           let mut peers_guard = peers.lock().await;
                           peers_guard.insert(peer_id, peer);
                           let _ = core_bridge.response(core_request_id, CoreOperationOutput::P2P(P2POperationOutput::PeerConnected(peer_entity))).await;
                       }));
                   }
               }
               else {
                   let mut peers_guard = self.peers.lock().await;
                   peers_guard.remove(&peer_id);
               }
            }

            for (peer_id, msg) in socket.channel_mut(MSG_CHANNEL_ID).receive() {
                let peers_guard = self.peers.lock().await;
                let Some(peer) = peers_guard.get(&peer_id) else {
                    continue;
                };

                let Ok(msg) = PeerMessageBody::decode(&msg[..]) else {
                    continue;
                };

                let request_id = msg.request_id;
                if let Some(response) = msg.response {
                    peer.msg_channel.notify_response(request_id.clone(), response).await;
                };

                let Some(request) = msg.request else {
                    continue;
                };

                if let Request::IntroduceRequest(request) = request {
                    if peers_guard.contains_key(&peer_id) {
                        log::warn!("Skip the peer since it already exists");
                        continue;
                    }

                    drop(peers_guard);

                    let core_bridge = self.core_bridge.clone();
                    let direct_message_channel =  DirectMessageChannel::new(peer_id.clone(), outbound_msg_sender.clone());
                    let current_user = current_user.clone();
                    let peer_id = peer_id.clone();
                    let peers = self.peers.clone();
                    let outbound_data_sender = outbound_data_sender.clone();
                    let outbound_thumbnail_sender = outbound_thumbnail_sender.clone();
                    let local_resource_repository = self.local_resource_repository.clone();
                    let request_id = request_id.clone();
                    handles.push(spawn(async move {
                        let peer = match WebRtcPeer::from_introduce_request(
                            current_user,
                            request_id,
                            request,
                            direct_message_channel,
                            core_bridge.clone(),
                            outbound_data_sender,
                            outbound_thumbnail_sender,
                            local_resource_repository
                        ).await {
                            Ok(peer) => peer,
                            Err(err) => {
                                log::error!("Failed to connect to peer {:?}", err);
                                return;
                            }
                        };

                        let mut peers = peers.lock().await;
                        let peer_entity = peer.peer.clone();
                        peers.insert(peer_id, peer);
                        let _ = core_bridge.response(core_request_id, CoreOperationOutput::P2P(P2POperationOutput::PeerConnected(peer_entity))).await;
                    }));

                    continue;
                };

                let peers_guard = self.peers.lock().await;
                if let Some(peer) = peers_guard.get(&peer_id) {
                    peer.process_request(request);
                }
            }

            for (peer, data) in socket.channel_mut(TRANSFER_RESOURCE_CHANNEL_ID).receive() {
                let Some(peer) = self.peers.lock().await.get(&peer) else {
                    continue;
                };
            }

            select! {
                // Restart this loop every 100 ms
                _ = (&mut timeout).fuse() => {
                    timeout.reset(Duration::from_millis(100));
                }
                _ = async {
                     for handle in handles.drain(..) {
                        if let Err(e) = handle.await {
                            log::error!("Error while joining async task: {:?}", e);
                        }
                    }
                }.fuse() => {
                    timeout.reset(Duration::from_millis(100));
                }
                // Or break if the message loop ends (disconnected, closed, etc.)
                _ = &mut loop_fut => {
                    break;
                }
            }
        }

        Ok(())
    }
}
