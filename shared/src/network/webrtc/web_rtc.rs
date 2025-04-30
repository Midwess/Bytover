use std::collections::HashMap;
use std::sync::{Arc, Weak};
use std::time::Duration;

use core_services::logger::ThrottleLogger;
use core_services::utils::number::ExponentialGrowth;
use futures_util::lock::Mutex;
use schema::devlog::rpc_signalling::server::{JoinMessage, Message};
use thiserror::Error;
use tokio::spawn;
use tokio::sync::OnceCell;
use tokio::task::JoinHandle;
use tokio::time::sleep;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;

use crate::app::nearby::finding_scope::FindingScope;
use crate::app::operations::p2p::P2POperationOutput;
use crate::app::operations::CoreOperationOutput;
use crate::entities::peer::Peer;
use crate::native::message_to_shell::MessageToShell;
use crate::{serialize, ShellRuntime};

use super::connection::{ConnectionWebRtc, ConnectionWebRtcErrors};
use super::peer::{PeerCommunication, PeerErrors};
use super::signalling::{RtcSignallingErrors, RtcsSignalling};
use super::throughput::ThroughputController;

#[derive(Debug, Error)]
pub enum WebRtcErrors {
    #[error("failedServerError to create peer connection {:?}", .0)]
    WebRTCServerError(#[from] webrtc::Error),
    #[error("failed to connect to signalling server {:?}", .0)]
    SignallingServerError(#[from] RtcSignallingErrors),
    #[error("failed to create connection {:?}", .0)]
    ConnectionError(#[from] ConnectionWebRtcErrors),
    #[error("failed to transfer data {:?}", .0)]
    TransferError(#[from] PeerErrors)
}

pub struct WebRtc {
    scopes: Arc<Mutex<Vec<FindingScope>>>,
    broadcast_handle: Arc<Mutex<Option<JoinHandle<()>>>>,
    peer: OnceCell<Peer>,
    pub(crate) connections: Mutex<HashMap<u128, OnceCell<Arc<PeerCommunication>>>>,
    signalling_client: OnceCell<Arc<RtcsSignalling>>,
    handle_signalling_message_join: Arc<Mutex<Option<JoinHandle<()>>>>,
    shell_runtime: OnceCell<Arc<dyn ShellRuntime>>,
    workdir: String,
    throughput_controller: Arc<ThroughputController>
}

impl Default for WebRtc {
    fn default() -> Self {
        Self::new(String::new())
    }
}

impl WebRtc {
    pub fn throughput_controller() -> Arc<ThroughputController> {
        Arc::new(ThroughputController::new(2 * 1024 * 1024, Duration::from_secs(10)))
    }

    pub fn new(workdir: String) -> Self {
        Self {
            peer: OnceCell::new(),
            shell_runtime: OnceCell::new(),
            scopes: Arc::new(Mutex::new(vec![])),
            broadcast_handle: Arc::new(Mutex::new(None)),
            connections: Mutex::new(HashMap::new()),
            signalling_client: OnceCell::new(),
            handle_signalling_message_join: Arc::new(Mutex::new(None)),
            workdir,
            throughput_controller: Self::throughput_controller()
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

    pub async fn start(
        self: &Arc<Self>,
        core_request_id: u32,
        peer: Peer,
        shell_runtime: Arc<dyn ShellRuntime>
    ) -> Result<(), WebRtcErrors> {
        let _ = self.peer.set(peer);
        let _ = self.shell_runtime.set(shell_runtime);

        log::info!(target: "rtc", "Starting signalling client");
        let signalling_client = RtcsSignalling::start().await?;
        let _ = self.signalling_client.set(Arc::new(signalling_client));

        self.broadcast().await?;

        self.handle_nearby_event(core_request_id).await?;

        Ok(())
    }

    pub async fn update_finding_scopes(&self, scopes: Vec<FindingScope>) -> Result<(), WebRtcErrors> {
        let mut current_scopes = self.scopes.lock().await;
        if current_scopes.eq(&scopes) {
            return Ok(());
        }

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
        let exponential_growth_delay = ExponentialGrowth::new(3, 0.1, 3, 35);
        let throttle_logger = ThrottleLogger::new("broadcast-task".to_string(), Duration::from_secs(15));
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
                else {
                    throttle_logger.log(format!("Broadcasting..., my id = {my_id}")).await;
                }

                sleep(delay).await;
            }
        }));

        Ok(())
    }

    pub async fn handle_nearby_event(self: &Arc<Self>, core_request_id: u32) -> Result<(), WebRtcErrors> {
        let mut subscription = self.signalling_client.get().unwrap().subscribe();
        let throttle_logger = ThrottleLogger::new("handle-nearby-event".to_string(), Duration::from_secs(15));
        let ns = "handle-nearby-event".to_string();
        while let Ok(message) = subscription.recv().await {
            let my_id = self.id();
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

                throttle_logger.log(format!("Received join message from {peer_id}")).await;

                let mut current_connections = self.connections.lock().await;
                if current_connections.contains_key(&peer_id) {
                    continue;
                }

                current_connections.insert(peer_id, OnceCell::new());

                let peer = self.peer().clone();
                let self_clone = self.clone();
                spawn(async move {
                    let connect_result = ConnectionWebRtc::offer(
                        from_scope,
                        peer,
                        peer_id,
                        self_clone.signalling_client.get().unwrap().clone(),
                        self_clone.shell_runtime().clone(),
                        self_clone.throughput_controller.clone(),
                        self_clone.workdir.clone()
                    )
                    .await;

                    self_clone.handle_connection(core_request_id, connect_result, peer_id).await;
                });

                continue;
            }

            if let Some(offer) = message.offer {
                if peer_id >= my_id {
                    log::info!(target: "broadcast", "Peer {:?} is not less than my id {:?}, reject offer", peer_id, my_id);
                    continue;
                }

                throttle_logger.log(format!("Received offer from {peer_id}")).await;

                let mut current_connections = self.connections.lock().await;
                if current_connections.contains_key(&peer_id) {
                    continue;
                }

                current_connections.insert(peer_id, OnceCell::new());

                let self_clone = self.clone();
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
                                self_clone.shell_runtime().clone(),
                                self_clone.throughput_controller.clone(),
                                self_clone.workdir.clone()
                            )
                            .await;

                            self_clone.handle_connection(core_request_id, connection, peer_id).await;
                        }
                        Err(e) => {
                            log::error!(target: "broadcast", "Error creating session description: {:?}", e);
                            let mut current_connections = self_clone.connections.lock().await;
                            current_connections.remove(&peer_id);
                        }
                    }
                });

                continue;
            }

            if let Some(left_message) = message.left_message {
                throttle_logger.log(format!("Received left message from {}", left_message.id)).await;

                let mut current_connections = self.connections.lock().await;
                if let Some(conn) = current_connections
                    .remove(&left_message.id.parse::<u128>().expect("Failed to parse peer id"))
                    .and_then(|conn| conn.get().cloned())
                {
                    let _ = conn.close().await;
                    log::info!(target: "broadcast", "Peer {:?} left", left_message.id);
                }

                continue;
            }
        }

        log::info!(target: "broadcast", "Unsubscribed from signalling messages");

        Ok(())
    }

    pub async fn get_connection(&self, peer_id: u128) -> Result<Weak<PeerCommunication>, WebRtcErrors> {
        let current_connections = self.connections.lock().await;
        let Some(connection) = current_connections.get(&peer_id) else {
            log::error!(target: "broadcast", "Connection not found for peer {:?}", peer_id);
            return Err(WebRtcErrors::ConnectionError(ConnectionWebRtcErrors::ConnectionNotFound))
        };

        let Some(connection) = connection.get() else {
            log::error!(target: "broadcast", "Connection not yet available for peer {:?}", peer_id);
            return Err(WebRtcErrors::ConnectionError(ConnectionWebRtcErrors::ConnectionNotFound))
        };

        Ok(Arc::downgrade(connection))
    }

    pub async fn handle_connection(
        self: &Arc<Self>,
        core_request_id: u32,
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
                            let mut current_connections = self_clone.connections.lock().await;
                            if let Some(conn) = current_connections.remove(&peer_id).and_then(|conn| conn.get().cloned()) {
                                let _ = conn.close().await;
                            }

                            log::info!(target: "broadcast", "Removed connection for peer {:?}", peer_id);
                        })
                    })
                });

                let peer_id = connection.peer_id;
                let current_connections = self.connections.lock().await;
                let msg = CoreOperationOutput::P2P(P2POperationOutput::PeerConnected(connection.peer.clone()));
                let _ = current_connections.get(&peer_id).unwrap().set(Arc::new(connection));
                drop(current_connections);
                self.shell_runtime()
                    .msg_from_native(serialize(&MessageToShell::HandleResponse(core_request_id, msg)))
                    .await;
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
