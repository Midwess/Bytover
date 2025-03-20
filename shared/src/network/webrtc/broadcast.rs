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
    id: String,
    connections: Mutex<HashMap<String, OnceCell<ConnectionWebRtc>>>,
    signalling_client: OnceCell<Arc<RtcsSignalling>>,
    handle_signalling_message_join: Arc<Mutex<Option<JoinHandle<()>>>>,
}

impl BroadcastWebRtc {
    pub fn new(scopes: Vec<String>) -> Self {
        Self {
            scopes,
            broadcast_handle: Arc::new(Mutex::new(None)),
            id: uuid::Uuid::new_v4().to_string(),
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
        let my_id = self.id.clone();
        let signalling_client = self.signalling_client.clone();
        let message = Message {
            scopes,
            from_id: my_id.clone(),
            join: Some(JoinMessage { id: my_id }),
            ..Default::default()
        };

        *broadcast_handle = Some(spawn(async move {
            loop {
                log::info!(target: "broadcast", "Broadcasting...");
                sleep(Duration::from_secs(random_number_in_range(5, 15) as u64)).await;

                if let Err(e) = signalling_client.get().unwrap().send(message.clone()).await {
                    log::error!(target: "broadcast", "Error sending message, ignored: {:?}", e);
                }
            }
        }));

        Ok(())
    }

    pub async fn handle_signalling_message(self: &Arc<Self>) -> Result<(), BroadcastWebRtcErrors> {
        let mut subscription = self.signalling_client.get().expect("Signalling client not initialized")
            .subscribe();

        if let Some(handle) = self.handle_signalling_message_join.lock().await.take() {
            handle.abort();
        }

        let self_clone = self.clone();
        *self.handle_signalling_message_join.lock().await = Some(spawn(async move {
            while let Ok(message) = subscription.recv().await {
                let my_id = self_clone.id.clone();
                if message.from_id.eq(&my_id) {
                    continue;
                }

                let peer_id = message.from_id.clone();

                if let Some(join) = message.join {
                    let mut current_connections= self_clone.connections.lock().await;
                    if current_connections.contains_key(&peer_id) {
                        continue;
                    }

                    current_connections.insert(peer_id.clone(), OnceCell::new());
                    drop(current_connections);

                    match ConnectionWebRtc::local(
                        my_id.clone(),
                        peer_id.clone(),
                        self_clone.signalling_client.get().unwrap().clone()
                    ).await {
                        Ok(connection) => {
                            let mut current_connections = self_clone.connections.lock().await;
                            current_connections.get_mut(&peer_id).unwrap().set(connection);
                        },
                        Err(e) => {
                            log::error!(target: "broadcast", "No connection to peer {:?} {:?}", peer_id, e);
                            let mut current_connections = self_clone.connections.lock().await;
                            current_connections.remove(&peer_id);
                        },
                    }
                }

                if let Some(offer) = message.offer {
                    let mut current_connections = self_clone.connections.lock().await;
                    if current_connections.contains_key(&peer_id) {
                        continue;
                    }

                    current_connections.insert(peer_id.clone(), OnceCell::new());

                    drop(current_connections);

                    match RTCSessionDescription::offer(offer.sdp) {
                        Ok(desc) => {
                            match ConnectionWebRtc::remote(my_id.clone(), peer_id.clone(), desc, self_clone.signalling_client.get().unwrap().clone()).await {
                                Ok(connection) => {
                                    let mut current_connections = self_clone.connections.lock().await;
                                    let _ = current_connections.get_mut(&peer_id).unwrap().set(connection);
                                },
                                Err(e) => {
                                    log::error!(target: "broadcast", "Error creating remote connection: {:?} {:?}", peer_id, e);
                                    let mut current_connections = self_clone.connections.lock().await;
                                    current_connections.remove(&peer_id);
                                }
                            }
                        },
                        Err(e) => log::error!(target: "broadcast", "Error creating session description: {:?}", e),
                    }
                }
            }
        }));

        Ok(())
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
