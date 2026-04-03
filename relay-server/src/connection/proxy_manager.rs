use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use futures_util::stream::FuturesUnordered;
use futures_util::StreamExt;
use thiserror::Error;
use tokio::sync::{Mutex, OnceCell};

use crate::connection::rtc::{RelayRtcClient, RelayRtcError};
use schema::devlog::bitbridge::DataChannel;
use str0m::Event;

#[derive(Debug, Error)]
pub enum ProxyManagerError {
    #[error("Relay RTC error: {0}")]
    RelayRtc(#[from] RelayRtcError),
    #[error("Proxy session not found: {0}")]
    SessionNotFound(String),
}

use crate::connection::proxy::ProxyInstance;

pub struct ProxyManager {
    proxies: Arc<Mutex<HashMap<String, Arc<ProxyInstance>>>>,
    running: AtomicBool,
}

impl ProxyManager {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            proxies: Arc::new(Mutex::new(HashMap::new())),
            running: AtomicBool::new(false),
        })
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    pub fn start(self: &Arc<Self>) {
        if self.is_running() {
            log::warn!("[relay-server] ProxyManager is already running");
            return;
        }

        self.running.store(true, Ordering::SeqCst);
        log::info!("[relay-server] ProxyManager started");
        
        // This server operates strictly via passive grpc invocations and doesn't need to loop on Signalling messages.
        // The event loops for the individual ProxyInstances are handled asynchronously when legs are joined.
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
        log::info!("[relay-server] ProxyManager stopped");
    }

    /// Handles a ConnectRequest. Creates or updates a proxy instance and returns the SdpAnswer.
    pub async fn handle_connect(
        self: &Arc<Self>,
        session_id: String,
        sdp_offer: String,
        channels: Vec<DataChannel>,
    ) -> Result<String, ProxyManagerError> {
        log::info!("[relay-server] Handling connect for session {}", session_id);
        
        let mut proxies = self.proxies.lock().await;

        if let Some(proxy) = proxies.get(&session_id) {
            log::info!("[relay-server] Joining existing ProxyInstance for session {}", session_id);
            let answer = proxy.proxy(sdp_offer, channels).await.map_err(ProxyManagerError::RelayRtc)?;
            Ok(answer)
        } else {
            log::info!("[relay-server] Creating new ProxyInstance for session {}", session_id);
            let proxy = ProxyInstance::new(session_id.clone());
            let answer = proxy.init(sdp_offer, channels, self.proxies.clone()).await.map_err(ProxyManagerError::RelayRtc)?;
            proxies.insert(session_id, proxy);
            Ok(answer)
        }
    }
}
