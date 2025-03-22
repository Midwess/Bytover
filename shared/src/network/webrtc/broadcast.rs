use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use core_services::utils::random_number_in_range;
use futures_util::lock::Mutex;
use schema::devlog::rpc_signalling::server::{IceCandidate, JoinMessage, Message};
use thiserror::Error;
use tokio::spawn;
use tokio::sync::OnceCell;
use tokio::task::JoinHandle;
use tokio::time::sleep;
use webrtc::ice_transport::ice_candidate::RTCIceCandidateInit;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;

use super::connection::{ConnectionWebRtc, ConnectionWebRtcErrors};
use super::signalling::{RtcSignallingErrors, RtcsSignalling};

#[derive(Debug, Error)]
pub enum BroadcastWebRtcErrors {
    #[error("failedServerError to create peer connection {:?}", .0)]
    WebRTCServerError(#[from] webrtc::Error),
    #[error("failed to connect to signalling server {:?}", .0)]
    SignallingServerError(#[from] RtcSignallingErrors),
    #[error("failed to create connection {:?}", .0)]
    ConnectionError(#[from] ConnectionWebRtcErrors)
}

pub struct BroadcastWebRtc {
    scopes: Vec<String>,
    broadcast_handle: Arc<Mutex<Option<JoinHandle<()>>>>,
    id: u128,
    connections: Mutex<HashMap<u128, OnceCell<ConnectionWebRtc>>>,
    signalling_client: OnceCell<Arc<RtcsSignalling>>,
    handle_signalling_message_join: Arc<Mutex<Option<JoinHandle<()>>>>
}

impl BroadcastWebRtc {
    pub fn new(scopes: Vec<String>) -> Self {
        Self {
            scopes,
            broadcast_handle: Arc::new(Mutex::new(None)),
            id: uuid::Uuid::new_v4().as_u128(),
            connections: Mutex::new(HashMap::new()),
            signalling_client: OnceCell::new(),
            handle_signalling_message_join: Arc::new(Mutex::new(None))
        }
    }

    pub async fn start(self: &Arc<Self>) -> Result<(), BroadcastWebRtcErrors> {
        let signalling_client = RtcsSignalling::start().await?;
        let _ = self.signalling_client.set(Arc::new(signalling_client));

        self.broadcast().await?;

        self.handle_signalling_message().await?;

        Ok(())
    }

    pub async fn broadcast(&self) -> Result<(), BroadcastWebRtcErrors> {
        let mut broadcast_handle = self.broadcast_handle.lock().await;
        if let Some(handle) = broadcast_handle.take() {
            handle.abort();
        }

        let scopes = self.scopes.clone();
        let my_id = self.id;
        let signalling_client = self.signalling_client.clone();
        let message = Message {
            scopes,
            from_id: my_id.to_string(),
            join: Some(JoinMessage { id: my_id.to_string() }),
            ..Default::default()
        };

        *broadcast_handle = Some(spawn(async move {
            log::info!(target: "broadcast", "{} Broadcasting...", my_id);
            loop {
                if let Err(e) = signalling_client.get().unwrap().send(message.clone()).await {
                    log::error!(target: "broadcast", "Error sending message, ignored: {:?}", e);
                }

                sleep(Duration::from_secs(random_number_in_range(5, 8) as u64)).await;
            }
        }));

        Ok(())
    }

    pub async fn handle_signalling_message(self: &Arc<Self>) -> Result<(), BroadcastWebRtcErrors> {
        if let Some(handle) = self.handle_signalling_message_join.lock().await.take() {
            handle.abort();
        }

        let self_clone = self.clone();
        *self.handle_signalling_message_join.lock().await = Some(spawn(async move {
            let mut subscription = self_clone.signalling_client.get().unwrap().subscribe();
            while let Ok(message) = subscription.recv().await {
                let my_id = self_clone.id;
                let peer_id = message.from_id_number();

                if let Some(join) = message.join {
                    if peer_id >= my_id {
                        continue;
                    }

                    let mut current_connections = self_clone.connections.lock().await;
                    if current_connections.contains_key(&peer_id) {
                        continue;
                    }

                    current_connections.insert(peer_id, OnceCell::new());
                    drop(current_connections);

                    let self_clone = self_clone.clone();
                    spawn(async move {
                        let connect_result =
                            ConnectionWebRtc::local(my_id, peer_id, self_clone.signalling_client.get().unwrap().clone()).await;

                        self_clone.handle_connection(connect_result).await;
                    });
                }

                if let Some(offer) = message.offer {
                    if peer_id <= my_id {
                        log::info!(target: "broadcast", "Peer {:?} is not greater than my id {:?}, reject offer", peer_id, my_id);
                        continue;
                    }

                    let mut current_connections = self_clone.connections.lock().await;
                    if current_connections.contains_key(&peer_id) {
                        log::info!(target: "broadcast", "Connection already exists for peer {:?}, reject offer", peer_id);
                        continue;
                    }

                    current_connections.insert(peer_id, OnceCell::new());

                    drop(current_connections);

                    let self_clone = self_clone.clone();
                    spawn(async move {
                        match RTCSessionDescription::offer(offer.sdp) {
                            Ok(desc) => {
                                let connection = ConnectionWebRtc::remote(
                                    my_id,
                                    peer_id,
                                    desc,
                                    self_clone.signalling_client.get().unwrap().clone()
                                )
                                .await;
                                self_clone.handle_connection(connection).await;
                            }
                            Err(e) => log::error!(target: "broadcast", "Error creating session description: {:?}", e)
                        }
                    });
                }
            }

            log::info!(target: "broadcast", "Unsubscribed from signalling messages");
        }));

        Ok(())
    }

    pub async fn handle_connection(self: &Arc<Self>, connect_result: Result<ConnectionWebRtc, ConnectionWebRtcErrors>) {
        match connect_result {
            Ok(connection) => {
                connection.on_disconnect({
                    let self_clone = self.clone();
                    Box::new(move || {
                        let self_clone = self_clone.clone();
                        Box::pin(async move {
                            log::info!(target: "broadcast", "Closing connection for peer {:?}", connection.peer_id);
                            let mut current_connections = self_clone.connections.lock().await;
                            log::info!(target: "broadcast", "Removing connection for peer {:?}", connection.peer_id);
                            current_connections.remove(&connection.peer_id);
                        })
                    })
                });

                let peer_id = connection.peer_id;
                let mut current_connections = self.connections.lock().await;
                let _ = current_connections.get_mut(&peer_id).unwrap().set(connection);
            }
            Err(e) => {
                log::error!(target: "broadcast", "Error creating connection: {:?}", e);
            }
        }
    }

    pub fn parse_ice_candidate(candidate: IceCandidate) -> RTCIceCandidateInit {
        // Parse the candidate string to extract needed information
        RTCIceCandidateInit {
            candidate: candidate.candidate,
            sdp_mid: Some(candidate.sdp_mid),
            sdp_mline_index: Some(candidate.sdp_mline_index as u16),
            username_fragment: None
        }
    }
}

impl Drop for BroadcastWebRtc {
    fn drop(&mut self) {
        let broadcast_handle = self.broadcast_handle.clone();
        let handle_signalling_message_join = self.handle_signalling_message_join.clone();
        spawn(async move {
            if let Some(handle) = broadcast_handle.lock().await.take() {
                handle.abort();
            }

            if let Some(handle) = handle_signalling_message_join.lock().await.take() {
                handle.abort();
            }
        });
    }
}
