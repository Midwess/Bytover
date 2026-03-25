use std::collections::HashMap;
use std::net::SocketAddr;
use futures_util::stream::FuturesUnordered;
use futures_util::StreamExt;
use str0m::net::Transmit;
use str0m::Candidate;
use thiserror::Error;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;

use crate::webrtc::client::WebRtcClient;
use crate::webrtc::signalling::SignalingClient;
use crate::webrtc::socket::SyncUdpSocket;

#[derive(Debug, Error)]
pub enum WebRtcServerError {
    #[error("Socket error: {0}")]
    Socket(#[from] std::io::Error),

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
        let local_addr = socket.local_addr();
        log::info!("[webrtc-server] UDP socket bound on {local_addr}");

        self.signalling.start();
        log::info!("[webrtc-server] Signalling background task started");

        let (outbound_tx, mut outbound_rx) = mpsc::unbounded_channel::<Transmit>();
        let mut buf = vec![0u8; 65535];

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
                    }

                    if let Some(offer) = msg.offer {
                        let ices = self.gathering_ices().await?;
                        let client_socket = socket.clone();
                        let signalling = self.signalling.clone();
                        let scopes = self.config.scopes.clone();
                        connect_futs.push(async move {
                            WebRtcClient::connect(offer, ices, client_socket, signalling, request_id).await
                        });
                    }
                }

                Some(result) = connect_futs.next() => {
                    match result {
                        Ok(client) => {
                            log::info!("[webrtc-server] Client {peer_id} connected");
                            self.clients.push(client);
                            log::info!("[webrtc-server] Active clients: {}", self.clients.len());
                        }
                        Err(e) => {
                            log::error!("[webrtc-server] Client {peer_id} connection failed: {e}");
                        }
                    }
                }

                result = socket.recv_from(&mut buf) => {
                    match result {
                        Ok((len, source)) => {
                            log::debug!("[webrtc-server] Received {} bytes from {}", len, source);
                        }
                        Err(e) => {
                            log::error!("[webrtc-server] UDP recv error: {e}");
                            break;
                        }
                    }
                }

                transmit = outbound_rx.recv() => {
                    if let Some(t) = transmit {
                        if let Err(e) = socket.send_to(&t.contents, t.destination).await {
                            log::warn!("[webrtc-server] UDP send error: {e}");
                        }
                    }
                }
            }
        }

        Ok(())
    }

    pub async fn gathering_ices(&self) -> Result<Vec<Candidate>, WebRtcServerError> {
        Ok(vec![])
    }
}
