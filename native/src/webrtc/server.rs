use std::collections::HashMap;
use std::net::SocketAddr;
use futures_util::stream::FuturesUnordered;
use futures_util::StreamExt;
use str0m::net::Transmit;
use thiserror::Error;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;

use crate::webrtc::client::WebRtcClient;
use crate::webrtc::ice::IceAgent;
use crate::webrtc::signalling::SignalingClient;
use crate::webrtc::socket::{SyncUdpSocket, SyncUdpSocketError};

#[derive(Debug, Error)]
pub enum WebRtcServerError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Socket error: {0}")]
    Socket(#[from] SyncUdpSocketError),

    #[error("Signalling error: {0}")]
    Signalling(#[from] crate::webrtc::signalling::SignallingError),

    #[error("str0m RTC error: {0}")]
    Rtc(#[from] str0m::error::RtcError),

    #[error("SDP parse error: {0}")]
    SdpParse(String),

    #[error("ICE candidate parse error: {0}")]
    IceParse(String),

    #[error("Unknown peer: {0}")]
    UnknownPeer(String),
}


pub struct WebRtcServerConfig {
    pub bind_addr: SocketAddr,
    pub signalling_host: String,
    pub signalling_port: u16,
    pub signalling_ssl: bool,
    pub scopes: Vec<String>,
    pub server_id: String,
    pub version: String,
}

pub struct WebRtcServer {
    config: WebRtcServerConfig,
    signalling: SignalingClient,
    clients: Vec<WebRtcClient>,
    addr_to_peer: HashMap<SocketAddr, String>,
}

impl WebRtcServer {
    pub fn new(config: WebRtcServerConfig) -> Self {
        let signalling = SignalingClient::new(
            config.signalling_host.clone(),
            config.signalling_port,
            config.signalling_ssl,
        );
        Self {
            config,
            signalling,
            clients: Default::default(),
            addr_to_peer: HashMap::new(),
        }
    }

    pub async fn run(&mut self) -> Result<(), WebRtcServerError> {
        let socket = SyncUdpSocket::new(UdpSocket::bind(self.config.bind_addr).await?);
        let local_addr = socket.local_addr()?;
        log::info!("[webrtc-server] UDP socket bound on {local_addr}");

        self.signalling.start();
        log::info!("[webrtc-server] Signalling background task started");

        let mut ice_agent: Option<IceAgent> = None;
        let mut connect_futs: FuturesUnordered<_> = FuturesUnordered::new();

        loop {
            tokio::select! {
                msg = self.signalling.next() => {
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

                    let msg_offer = msg.offer;

                    if let Some(offer) = msg_offer {
                        if ice_agent.is_none() {
                            let config = self.signalling
                                .fetch_relay_config(&self.config.server_id)
                                .await?;
                            log::info!(
                                "[webrtc-server] IceAgent created with {} STUN URLs",
                                config.urls.len()
                            );
                            ice_agent = Some(IceAgent::new(config));
                        }
                        let agent = ice_agent.as_ref().unwrap().clone();

                        let client_socket = socket.clone();
                        let signalling = self.signalling.clone();
                        connect_futs.push(async move {
                            WebRtcClient::connect(offer, client_socket, signalling, request_id, agent).await
                        });
                    }
                }

                Some(result) = connect_futs.next() => {
                    match result {
                        Ok(client) => {
                            log::info!("[webrtc-server] Client connected");
                            self.clients.push(client);
                            log::info!("[webrtc-server] Active clients: {}", self.clients.len());
                        }
                        Err(e) => {
                            log::error!("[webrtc-server] Client connection failed: {e}");
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
