use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Weak};
use std::time::Duration;

use futures_util::stream::FuturesUnordered;
use futures_util::StreamExt;
use thiserror::Error;
use tokio::sync::{Mutex, OnceCell};

use shared::app::operations::p2p::P2POperationOutput;
use shared::app::operations::CoreOperationOutput;
use shared::entities::local_resource::LocalResource;
use shared::entities::peer::Peer as PeerEntity;
use shared::errors::CoreError;
use shared::repository::local_resource::LocalResourceRepository;
use shared::shell::api::CoreRequest;

use crate::config::{get_signalling_server_http_url_for_route, get_signalling_server_ws_url_for_route};
use crate::webrtc::client::{WebRtcClient, WebRtcClientError};
use crate::webrtc::signalling::SignalingClient;

#[derive(Debug, Error)]
pub enum WebRtcServerError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Signalling error: {0}")]
    Signalling(String),

    #[error("str0m RTC error: {0}")]
    Rtc(#[from] str0m::error::RtcError),

    #[error("SDP parse error: {0}")]
    SdpParse(String),

    #[error("Unknown peer: {0}")]
    UnknownPeer(String),

    #[error("Peer not found: {0}")]
    PeerNotFound(String),

    #[error("Client error: {0}")]
    Client(String)
}

impl From<WebRtcClientError> for WebRtcServerError {
    fn from(err: WebRtcClientError) -> Self {
        WebRtcServerError::Client(format!("{err}"))
    }
}

impl From<WebRtcServerError> for CoreError {
    fn from(err: WebRtcServerError) -> Self {
        CoreError::Network(format!("WebRtcServer {err:?}"))
    }
}

pub struct WebRtcServer {
    clients: Mutex<HashMap<String, Weak<WebRtcClient>>>,
    resource_repo: Arc<dyn LocalResourceRepository>,
    current_user: OnceCell<PeerEntity>,
    core_request: OnceCell<CoreRequest>,
    running: AtomicBool
}

impl WebRtcServer {
    pub fn new(resource_repo: Arc<dyn LocalResourceRepository>) -> Arc<Self> {
        Arc::new(Self {
            clients: Mutex::new(HashMap::new()),
            resource_repo,
            current_user: Default::default(),
            core_request: Default::default(),
            running: AtomicBool::new(false)
        })
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    async fn get_client(&self, peer_id: &str) -> Result<Arc<WebRtcClient>, WebRtcServerError> {
        let mut clients = self.clients.lock().await;
        let result = clients
            .get(peer_id)
            .cloned()
            .and_then(|client| client.upgrade())
            .ok_or_else(|| WebRtcServerError::PeerNotFound(peer_id.to_string()));

        if result.is_err() {
            clients.remove(peer_id);
        }

        result
    }

    pub async fn get_peer(&self, peer_id: &str) -> Option<PeerEntity> {
        let mut clients = self.clients.lock().await;
        let client = clients.get(peer_id).cloned().and_then(|client| client.upgrade());

        if client.is_none() {
            clients.remove(peer_id);
        }

        client.and_then(|client| client.peer_entity())
    }

    pub async fn cancel_session(&self, peer_id: String, session_id: u64) -> Result<(), WebRtcServerError> {
        let client = self.get_client(&peer_id).await?;
        client.cancel_transfer(session_id).await;
        tokio::time::sleep(Duration::from_millis(150)).await;
        client.disconnect();
        Ok(())
    }

    pub async fn broadcast_cancel_session(&self, session_id: u64, resource_id: Option<u64>) -> Result<(), WebRtcServerError> {
        let mut clients = self.clients.lock().await;
        let mut stale_peers = Vec::new();
        let mut session_clients = Vec::new();

        for (peer_id, client) in clients.iter() {
            let Some(client) = client.upgrade() else {
                stale_peers.push(peer_id.clone());
                continue;
            };

            if client.session_id() == Some(session_id) {
                session_clients.push(client);
            }
        }

        for peer_id in stale_peers {
            clients.remove(&peer_id);
        }

        drop(clients);

        log::info!(
            "[webrtc-server] Broadcasting cancel for session {} to {} matching peers",
            session_id,
            session_clients.len()
        );

        for client in &session_clients {
            if let Some(resource_id) = resource_id {
                client.cancel_resource_transfer(session_id, resource_id).await;
            } else {
                client.cancel_transfer(session_id).await;
            }
        }

        if resource_id.is_none() && !session_clients.is_empty() {
            tokio::time::sleep(Duration::from_millis(150)).await;
            for client in session_clients {
                client.disconnect();
            }
        }

        Ok(())
    }

    pub async fn send_session_detail(
        &self,
        peer_id: String,
        request_id: String,
        session_message: Option<schema::devlog::bitbridge::P2pTransferSessionMessage>,
        resources: Option<Vec<LocalResource>>,
        error: Option<CoreError>
    ) -> Result<(), WebRtcServerError> {
        let client = self.get_client(&peer_id).await?;
        client.send_session_detail_response(request_id, session_message, resources, error).await?;
        Ok(())
    }

    pub async fn stream_resource_to_peer(
        &self,
        peer_id: String,
        session_id: u64,
        transfer_id: u16,
        resource: LocalResource
    ) -> Result<(), WebRtcServerError> {
        let client = self.get_client(&peer_id).await?;
        client.stream_resource(session_id, transfer_id, resource).await?;
        Ok(())
    }

    pub async fn send_resource_notification(&self, session_id: u64, resource: LocalResource) -> Result<(), WebRtcServerError> {
        for client in self.clients.lock().await.values() {
            let Some(client) = client.upgrade() else {
                continue;
            };

            client.send_resource_notification(session_id, resource.clone()).await?;
        }

        Ok(())
    }

    pub fn stop(&self) -> Result<(), WebRtcServerError> {
        self.running.store(false, Ordering::SeqCst);
        Ok(())
    }

    pub async fn start(self: &Arc<Self>, core_request: CoreRequest, current_user: PeerEntity) -> Result<(), WebRtcServerError> {
        if self.is_running() {
            log::info!("[webrtc-server] Already running");
            core_request.response(CoreOperationOutput::P2P(P2POperationOutput::AlreadyRunning)).await;
            return Ok(());
        }

        self.running.store(true, Ordering::SeqCst);

        let _ = self.core_request.set(core_request.clone());
        let _ = self.current_user.set(current_user.clone());

        log::info!("[webrtc-server] Starting with peer = {:?}", current_user.id);

        let Some(key) = current_user.signalling_id.clone() else {
            return Err(WebRtcServerError::Signalling(format!(
                "No signalling id for peer {}",
                current_user.id
            )))
        };

        let Some(signalling_route) = current_user.signalling_route.clone() else {
            return Err(WebRtcServerError::Signalling(format!(
                "No signalling route for peer {}",
                current_user.id
            )));
        };

        let mut signalling = SignalingClient::new(
            get_signalling_server_ws_url_for_route(&signalling_route),
            get_signalling_server_http_url_for_route(&signalling_route)
        );
        signalling.start(key.clone()).await;
        log::info!("[webrtc-server] Signalling background task started");

        let mut connect_futs: FuturesUnordered<_> = FuturesUnordered::new();
        let mut run_handles: FuturesUnordered<_> = FuturesUnordered::new();
        let resource_repo = self.resource_repo.clone();
        let current_user = current_user.clone();

        loop {
            if !self.is_running() {
                return Ok(());
            }

            tokio::select! {
                msg = signalling.next() => {
                    let msg = match msg {
                        Ok(m) => m,
                        Err(e) => {
                            log::warn!("[webrtc-server] Signalling error: {e:?}");
                            continue;
                        }
                    };

                    let Some(request_id) = msg.request_id.clone() else {
                        continue;
                    };

                    log::info!("Received offer {:?}", msg.offer);
                    if let Some(offer) = msg.offer {
                        let user = current_user.clone();
                        let repo = resource_repo.clone();
                        let sig_sender = signalling.get_sender().expect("Signalling sender must be available");
                        let rid = request_id.clone();
                        let off = offer.clone();
                        let sess = msg.session_id.clone().unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

                        connect_futs.push(async move {
                            let result = WebRtcClient::connect(
                                user.clone(),
                                off,
                                sess,
                                sig_sender,
                                rid,
                                repo,
                            )
                            .await;

                            match &result {
                                Ok(_) => log::info!("[webrtc-server] Client connected successfully"),
                                Err(e) => log::error!("[webrtc-server] Client connection failed: {:?}", e),
                            }

                            if let Ok(client) = result {
                                Some((Arc::new(client), user))
                            } else {
                                None
                            }
                        });
                    }
                }

                Some(result) = connect_futs.next(), if !connect_futs.is_empty() => {
                    match result {
                        Some((client, _user)) => {
                            let peer_id = client.peer_id().unwrap_or_default();
                            log::info!("[webrtc-server] Client {} connected, registering", peer_id);
                            self.clients.lock().await.insert(peer_id.clone(), Arc::downgrade(&client));

                            client.start_core_stream(self.core_request.get().unwrap().clone());
                            let client_for_run = client.clone();
                            let peer_id_for_run = peer_id.clone();

                            run_handles.push(tokio::spawn(async move {
                                log::info!("[webrtc-server] Spawning run loop");
                                if let Err(e) = client_for_run.run().await {
                                    log::error!("[webrtc-server] Client run error: {e}");
                                }
                                peer_id_for_run
                            }));
                        }
                        None => {
                            continue;
                        }
                    }
                }

                Some(res) = run_handles.next(), if !run_handles.is_empty() => {
                    match res {
                        Ok(peer_id) => {
                            log::info!("[webrtc-server] Client {peer_id} run loop finished");
                            let peer = {
                                let clients = self.clients.lock().await;
                                clients.get(&peer_id).and_then(|c| c.upgrade()).and_then(|c| c.peer_entity())
                            };
                            self.clients.lock().await.remove(&peer_id);
                            if let Some(p) = peer {
                                if let Some(req) = self.core_request.get() {
                                    let _ = req.response(CoreOperationOutput::P2P(P2POperationOutput::PeerDisconnected(p))).await;
                                }
                            }
                        }
                        Err(e) => {
                            log::error!("[webrtc-server] Client run task failed to join: {e}");
                        }
                    }
                }
            }
        }
    }
}
