use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Weak};

use futures_util::stream::FuturesUnordered;
use futures_util::StreamExt;
use thiserror::Error;
use tokio::sync::Mutex;

use crate::connection::rtc::RelayRtcError;
use schema::devlog::bitbridge::DataChannel;

#[derive(Debug, Error)]
pub enum ProxyManagerError {
    #[error("Relay RTC error: {0}")]
    RelayRtc(#[from] RelayRtcError)
}

use crate::connection::proxy::ProxyInstance;

/// Manages proxy instances. The HashMap only stores `Weak<ProxyInstance>` references.
/// The `start()` run loop holds the only strong `Arc<ProxyInstance>` references via
/// spawned tasks in `FuturesUnordered`. When a proxy's run loop finishes, the strong
/// reference is dropped and the Weak in the map becomes invalid.
pub struct ProxyManager {
    proxies: Mutex<HashMap<String, Weak<ProxyInstance>>>,
    run_tx: Mutex<Option<tokio::sync::mpsc::UnboundedSender<Arc<ProxyInstance>>>>,
    running: AtomicBool,
    public_ipv4: Ipv4Addr,
}

impl ProxyManager {
    pub fn new(public_ipv4: Ipv4Addr) -> Arc<Self> {
        Arc::new(Self {
            proxies: Mutex::new(HashMap::new()),
            run_tx: Mutex::new(None),
            running: AtomicBool::new(false),
            public_ipv4,
        })
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Starts the ProxyManager run loop. This is an async loop (like WebRtcServer::start)
    /// that owns all proxy run tasks via FuturesUnordered and cleans up when they finish.
    pub async fn start(self: &Arc<Self>) {
        if self.is_running() {
            log::warn!("[relay-server] ProxyManager is already running");
            return;
        }

        self.running.store(true, Ordering::SeqCst);
        log::info!("[relay-server] ProxyManager started");

        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Arc<ProxyInstance>>();
        *self.run_tx.lock().await = Some(tx);

        let mut run_handles: FuturesUnordered<tokio::task::JoinHandle<String>> = FuturesUnordered::new();

        loop {
            if !self.is_running() {
                log::info!("[relay-server] ProxyManager stopping, exiting run loop");
                return;
            }

            tokio::select! {
                // Receive newly created proxies from handle_connect and spawn their run loops
                Some(proxy) = rx.recv() => {
                    let session_id = proxy.session_id.clone();
                    log::info!("[relay-server] Spawning run loop for proxy session {session_id}");
                    run_handles.push(tokio::spawn(async move {
                        proxy.run().await
                    }));
                }

                // Collect finished proxy run loops and clean up the map
                Some(res) = run_handles.next(), if !run_handles.is_empty() => {
                    match res {
                        Ok(session_id) => {
                            log::info!("[relay-server] Proxy session {session_id} run loop finished, removing from map");
                            self.proxies.lock().await.remove(&session_id);
                        }
                        Err(e) => {
                            log::error!("[relay-server] Proxy run task failed to join: {e}");
                        }
                    }
                }
            }
        }
    }

    /// Handles a ConnectRequest. Creates or updates a proxy instance and returns the SdpAnswer.
    /// New proxies are sent to the `start()` run loop for lifecycle management.
    pub async fn handle_connect(
        self: &Arc<Self>,
        session_id: String,
        sdp_offer: String,
        channels: Vec<DataChannel>,
    ) -> Result<String, ProxyManagerError> {
        log::info!("[relay-server] Handling connect for session {}", session_id);

        let (proxy, is_new) = {
            let mut proxies = self.proxies.lock().await;
            match proxies.get(&session_id).and_then(|w| w.upgrade()) {
                Some(existing) => (existing, false),
                None => {
                    proxies.remove(&session_id);
                    let proxy = ProxyInstance::new(session_id.clone(), self.public_ipv4);
                    proxies.insert(session_id.clone(), Arc::downgrade(&proxy));
                    (proxy, true)
                }
            }
        };

        if !is_new {
            log::info!("[relay-server] Joining existing ProxyInstance for session {}", session_id);
            let answer = proxy.proxy(sdp_offer, channels).await.map_err(ProxyManagerError::RelayRtc)?;
            return Ok(answer);
        }

        log::info!("[relay-server] Creating new ProxyInstance for session {}", session_id);
        let answer = match proxy.init(sdp_offer, channels).await {
            Ok(a) => a,
            Err(e) => {
                self.proxies.lock().await.remove(&session_id);
                return Err(ProxyManagerError::RelayRtc(e));
            }
        };

        if let Some(tx) = self.run_tx.lock().await.as_ref() {
            let _ = tx.send(proxy);
        } else {
            log::error!("[relay-server] ProxyManager run loop not started, cannot spawn proxy for {session_id}");
            self.proxies.lock().await.remove(&session_id);
        }

        Ok(answer)
    }
}
