use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use core_services::utils::number::ExponentialGrowth;
use futures_util::lock::Mutex;
use schema::devlog::rpc_signalling::server::{JoinMessage, Message};
use thiserror::Error;
use tokio::spawn;
use tokio::sync::OnceCell;
use tokio::task::JoinHandle;
use tokio::time::sleep;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;

use crate::app::transfer::finding_scope::FindingScope;
use crate::entities::peer::Peer;
use crate::ShellRuntime;

use super::connection::{ConnectionWebRtc, ConnectionWebRtcErrors};
use super::peer::PeerCommunication;
use super::signalling::{RtcSignallingErrors, RtcsSignalling};

#[derive(Debug, Error)]
pub enum WebRtcErrors {
    #[error("failedServerError to create peer connection {:?}", .0)]
    WebRTCServerError(#[from] webrtc::Error),
    #[error("failed to connect to signalling server {:?}", .0)]
    SignallingServerError(#[from] RtcSignallingErrors),
    #[error("failed to create connection {:?}", .0)]
    ConnectionError(#[from] ConnectionWebRtcErrors)
}

pub struct WebRtc {
    scopes: Arc<Mutex<Vec<FindingScope>>>,
    broadcast_handle: Arc<Mutex<Option<JoinHandle<()>>>>,
    peer: OnceCell<Peer>,
    connections: Mutex<HashMap<u128, OnceCell<PeerCommunication>>>,
    signalling_client: OnceCell<Arc<RtcsSignalling>>,
    handle_signalling_message_join: Arc<Mutex<Option<JoinHandle<()>>>>,
    shell_runtime: OnceCell<Arc<dyn ShellRuntime>>
}

impl Default for WebRtc {
    fn default() -> Self {
        Self::new()
    }
}

impl WebRtc {
    pub fn new() -> Self {
        Self {
            peer: OnceCell::new(),
            shell_runtime: OnceCell::new(),
            scopes: Arc::new(Mutex::new(vec![])),
            broadcast_handle: Arc::new(Mutex::new(None)),
            connections: Mutex::new(HashMap::new()),
            signalling_client: OnceCell::new(),
            handle_signalling_message_join: Arc::new(Mutex::new(None))
        }
    }

    pub fn peer(&self) -> &Peer {
        self.peer.get().expect("Peer is not set")
    }

    pub fn id(&self) -> u128 {
        self.peer().id.parse::<u128>().expect("Failed to parse peer id, the peer id must be u128")
    }

    pub fn shell_runtime(&self) -> &Arc<dyn ShellRuntime> {
        self.shell_runtime.get().expect("Shell runtime is not set")
    }

    pub async fn start(self: &Arc<Self>, peer: Peer, shell_runtime: Arc<dyn ShellRuntime>) -> Result<(), WebRtcErrors> {
        let _ = self.peer.set(peer);
        let _ = self.shell_runtime.set(shell_runtime);

        let signalling_client = RtcsSignalling::start().await?;
        let _ = self.signalling_client.set(Arc::new(signalling_client));

        self.broadcast().await?;

        self.handle_signalling_message().await?;

        Ok(())
    }

    pub async fn update_finding_scopes(&self, scopes: Vec<FindingScope>) -> Result<(), WebRtcErrors> {
        let mut current_scopes = self.scopes.lock().await;
        current_scopes.clear();
        current_scopes.extend(scopes);

        log::info!(target: "broadcast", "Updated finding scopes: {:?}", current_scopes);

        Ok(())
    }

    pub async fn add_scope(&self, scope: FindingScope) {
        log::info!(target: "broadcast", "Adding scope: {}", scope.as_string());
        let mut scopes = self.scopes.lock().await;
        scopes.push(scope);
    }

    pub async fn broadcast(&self) -> Result<(), WebRtcErrors> {
        let mut broadcast_handle = self.broadcast_handle.lock().await;
        if let Some(handle) = broadcast_handle.take() {
            handle.abort();
        }

        let my_id = self.id();
        let signalling_client = self.signalling_client.clone();

        let scopes = self.scopes.clone();
        let exponential_growth_delay = ExponentialGrowth::new(3, 0.25, 3, 35);
        *broadcast_handle = Some(spawn(async move {
            loop {
                let delay = Duration::from_secs(exponential_growth_delay.next() as u64);
                let scopes = scopes.lock().await.clone();
                if scopes.is_empty() {
                    log::info!(target: "broadcast", "No scopes to broadcast, skipping...");
                    drop(scopes);
                    sleep(Duration::from_secs(1)).await;
                    continue;
                }

                let message = Message {
                    scopes: scopes.iter().map(|scope| scope.as_string()).collect(),
                    from_id: my_id.to_string(),
                    join: Some(JoinMessage { id: my_id.to_string() }),
                    ..Default::default()
                };

                if let Err(e) = signalling_client.get().unwrap().send(message.clone()).await {
                    log::error!(target: "broadcast", "Error sending message, ignored: {:?}", e);
                }

                sleep(delay).await;
            }
        }));

        Ok(())
    }

    pub async fn handle_signalling_message(self: &Arc<Self>) -> Result<(), WebRtcErrors> {
        if let Some(handle) = self.handle_signalling_message_join.lock().await.take() {
            handle.abort();
        }

        let self_clone = self.clone();
        *self.handle_signalling_message_join.lock().await = Some(spawn(async move {
            let mut subscription = self_clone.signalling_client.get().unwrap().subscribe();
            while let Ok(message) = subscription.recv().await {
                let my_id = self_clone.id();
                let peer_id = message.from_id_number();
                if let Some(to_id) = message.to_id_number() {
                    if to_id != my_id {
                        continue;
                    }
                }

                let Some(from_scope) = message.from_scope.and_then(FindingScope::from_string) else {
                    log::error!(target: "broadcast", "No from scope found");
                    continue;
                };

                if message.join.is_some() {
                    if peer_id <= my_id {
                        continue;
                    }

                    let mut current_connections = self_clone.connections.lock().await;
                    if current_connections.contains_key(&peer_id) {
                        continue;
                    }

                    current_connections.insert(peer_id, OnceCell::new());
                    drop(current_connections);

                    let self_clone = self_clone.clone();
                    let peer = self_clone.peer().clone();
                    spawn(async move {
                        let connect_result = ConnectionWebRtc::offer(
                            from_scope,
                            peer,
                            peer_id,
                            self_clone.signalling_client.get().unwrap().clone(),
                            self_clone.shell_runtime().clone()
                        )
                        .await;

                        self_clone.handle_connection(connect_result, peer_id).await;
                    });

                    continue;
                }

                if let Some(offer) = message.offer {
                    if peer_id >= my_id {
                        log::info!(target: "broadcast", "Peer {:?} is not less than my id {:?}, reject offer", peer_id, my_id);
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
                        let from_scope = from_scope.clone();
                        match RTCSessionDescription::offer(offer.sdp) {
                            Ok(desc) => {
                                let connection = ConnectionWebRtc::accept_offer(
                                    from_scope,
                                    self_clone.peer().clone(),
                                    peer_id,
                                    desc,
                                    self_clone.signalling_client.get().unwrap().clone(),
                                    self_clone.shell_runtime().clone()
                                )
                                .await;
                                self_clone.handle_connection(connection, peer_id).await;
                            }
                            Err(e) => {
                                log::error!(target: "broadcast", "Error creating session description: {:?}", e);
                                let mut current_connections = self_clone.connections.lock().await;
                                current_connections.remove(&peer_id);
                            }
                        }
                    });
                }

                if let Some(left_message) = message.left_message {
                    let mut current_connections = self_clone.connections.lock().await;
                    current_connections.remove(&left_message.id.parse::<u128>().expect("Failed to parse peer id"));
                }
            }

            log::info!(target: "broadcast", "Unsubscribed from signalling messages");
        }));

        Ok(())
    }

    pub async fn handle_connection(
        self: &Arc<Self>,
        connect_result: Result<PeerCommunication, ConnectionWebRtcErrors>,
        peer_id: u128
    ) {
        match connect_result {
            Ok(connection) => {
                connection.on_disconnect({
                    let self_clone = self.clone();
                    let peer_id = connection.peer_id;
                    Box::new(move || {
                        let self_clone = self_clone.clone();
                        Box::pin(async move {
                            log::info!(target: "broadcast", "Closing connection for peer {:?}", peer_id);
                            let mut current_connections = self_clone.connections.lock().await;
                            log::info!(target: "broadcast", "Removing connection for peer {:?}", peer_id);
                            current_connections.remove(&peer_id);
                        })
                    })
                });

                let peer_id = connection.peer_id;
                let current_connections = self.connections.lock().await;
                let _ = current_connections.get(&peer_id).unwrap().set(connection);
            }
            Err(e) => {
                let mut current_connections = self.connections.lock().await;
                current_connections.remove_entry(&peer_id);
                log::error!(target: "broadcast", "Error creating connection: {:?}", e);
            }
        }
    }
}

impl Drop for WebRtc {
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
