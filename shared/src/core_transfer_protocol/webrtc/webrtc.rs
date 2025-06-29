use std::collections::HashMap;
use std::sync::{Arc};
use tokio::sync::Mutex;
use futures::select;
use std::time::Duration;
use futures_timer::Delay;
use futures_util::FutureExt;
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
use crate::entities::peer::Peer as PeerEntity;

pub static MSG_CHANNEL_ID: usize = 0;
pub static TRANSFER_CHANNEL_ID: usize = 1;

pub struct WebRtc {
    peers: Arc<Mutex<HashMap<PeerId, WebRtcPeer>>>,
    core_bridge: Arc<dyn CoreBridge>,
    addr: String,
}

impl WebRtc {
    fn new(core_bridge: Arc<dyn CoreBridge>, addr: String) -> Self {
        Self {
            peers: Default::default(),
            core_bridge,
            addr
        }
    }

    async fn start(&self, core_request_id: u32, current_user: PeerEntity) -> Result<(), WebRtcErrors> {
        let signaller_builder = Arc::new(WebSignallerBuilder::new());
        let (mut socket, loop_fut) = WebRtcSocket::builder(self.addr.clone())
            .signaller_builder(signaller_builder.clone())
            .add_reliable_channel()
            .add_reliable_channel()
            .build();

        let loop_fut = loop_fut.fuse();
        futures::pin_mut!(loop_fut);
        let timeout = Delay::new(Duration::from_millis(100));
        futures::pin_mut!(timeout);

        let outbound_msg_sender = Arc::new(Mutex::new(socket.channel(MSG_CHANNEL_ID).sender_clone()));
        let direct_message_channel = DirectMessageChannel::new(outbound_msg_sender);
        
        loop {
            for (peer_id, state) in socket.try_update_peers()? {
               if state == matchbox_socket::PeerState::Connected {
                   let peers = self.peers.lock().await;
                   if peers.contains_key(&peer_id) {
                       continue;
                   }

                   if peer_id < current_user.peer_id() {
                       let direct_message_channel = direct_message_channel.clone();
                       let core_bridge = self.core_bridge.clone();
                       let peers = self.peers.clone();
                       let current_user = current_user.clone();
                       spawn(async move {
                           let peer = match WebRtcPeer::new(
                               current_user.clone(),
                               peer_id,
                               direct_message_channel,
                               core_bridge.clone(),
                           ).await {
                               Ok(peer) => peer,
                               Err(err) => {
                                   log::error!("Failed to connect to peer {:?}", err);
                                   return;
                               }
                           };

                           let peer_entity = peer.peer.clone();
                           let mut peers = peers.lock().await;
                           peers.insert(peer_id, peer);
                           let _ = core_bridge.response(core_request_id, CoreOperationOutput::P2P(P2POperationOutput::PeerConnected(peer_entity))).await;
                       });
                   }
               }
               else {
                   let mut peers = self.peers.lock().await;
                   peers.remove(&peer_id);
               }
            }

            for (peer_id, msg) in socket.channel_mut(MSG_CHANNEL_ID).receive() {
                let peers = self.peers.lock().await;
                let Some(peer) = peers.get(&peer_id) else {
                    continue;
                };

                let Ok(msg) = PeerMessageBody::decode(&msg[..]) else {
                    continue;
                };

                let Some(request) = msg.request else {
                    continue;
                };

                if let Request::IntroduceRequest(request) = request {
                    if peers.contains_key(&peer_id) {
                        continue;
                    }

                    let core_bridge = self.core_bridge.clone();
                    let direct_message_channel = direct_message_channel.clone();
                    let current_user = current_user.clone();
                    let peer_id = peer_id.clone();
                    let peers = self.peers.clone();
                    spawn(async move {
                        let peer = match WebRtcPeer::from_introduce_request(
                            current_user,
                            peer_id,
                            request,
                            direct_message_channel,
                            core_bridge.clone(),
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
                    });

                    continue;
                };

                if let Some(peer) = peers.get(&peer_id) {
                    peer.process_request(request);
                }
            }

            for (peer, data) in socket.channel_mut(TRANSFER_CHANNEL_ID).receive() {
                let Some(peer) = self.peers.lock().await.get(&peer) else {
                    continue;
                };
            }

            select! {
                // Restart this loop every 100 ms
                _ = (&mut timeout).fuse() => {
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
