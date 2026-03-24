use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

use str0m::change::SdpOffer;
use str0m::ice::IceAgent;
use str0m::net::{Protocol, Receive, Transmit};
use str0m::{Candidate, Event, IceConnectionState, Input, Output, Rtc, RtcConfig};
use thiserror::Error;
use tokio::net::UdpSocket;

use schema::devlog::rpc_signalling::server::OfferMessage;

#[derive(Debug, Error)]
pub enum WebRtcClientError {
    #[error("Rtc error: {0}")]
    Rtc(#[from] str0m::error::RtcError),

    #[error("SDP parse error: {0}")]
    SdpParse(String),

    #[error("ICE parse error: {0}")]
    IceParse(String),
}

pub struct InboundUdp {
    pub data: Vec<u8>,
    pub source: SocketAddr,
}

pub struct WebRtcClient {
    rtc: Rtc,
    socket: Arc<UdpSocket>,
    local_addr: SocketAddr,
}

impl WebRtcClient {
    pub async fn connect(
        offer_message: OfferMessage,
        gathered_ices: Vec<Candidate>,
        socket: Arc<UdpSocket>,
        local_addr: SocketAddr,
    ) -> Result<Self, WebRtcClientError> {
        let mut rtc = RtcConfig::new()
            .set_ice_lite(true)
            .build(Instant::now());

        for candidate in gathered_ices {
            rtc.add_remote_candidate(candidate);
        }

        let offer = SdpOffer::from_sdp_string(&offer_message.sdp)
            .map_err(|e| WebRtcClientError::SdpParse(format!("{e}")))?;

        let _answer = rtc
            .sdp_api()
            .accept_offer(offer)
            .map_err(WebRtcClientError::Rtc)?;

        let mut client = Self {
            rtc,
            socket,
            local_addr,
        };

        loop {
            let connected = client.str0m_handle().await?;
            if connected {
                break;
            }
        }

        Ok(client)
    }

    async fn str0m_handle(&mut self) -> Result<bool, WebRtcClientError> {
        let mut buf = vec![0u8; 65536];

        match self.rtc.poll_output()? {
            Output::Transmit(t) => {
                self.socket
                    .send_to(&t.contents, t.destination)
                    .await
                    .map_err(|e| WebRtcClientError::IceParse(format!("send error: {e}")))?;
            }
            Output::Event(event) => {
                match event {
                    Event::Connected => {
                        log::info!("[webrtc-client] Connected");
                        return Ok(true);
                    }
                    Event::IceConnectionStateChange(state) => {
                        log::info!("[webrtc-client] ICE state: {:?}", state);
                        if matches!(state, IceConnectionState::Disconnected) {
                            log::info!("[webrtc-client] Peer disconnected");
                            return Ok(true);
                        }
                    }
                    Event::ChannelOpen(id, label) => {
                        log::info!(
                            "[webrtc-client] Data channel opened: {label} (id={id:?})"
                        );
                    }
                    Event::ChannelData(data) => {
                        log::debug!(
                            "[webrtc-client] Channel data: {} bytes on {:?}",
                            data.data.len(),
                            data.id
                        );
                    }
                    Event::ChannelClose(id) => {
                        log::info!("[webrtc-client] Data channel closed: {id:?}");
                    }
                    _ => {}
                }
            }
            Output::Timeout(deadline) => {
                let now = Instant::now();
                if deadline > now {
                    tokio::time::sleep(deadline - now).await;
                }
                let _ = self.rtc.handle_input(Input::Timeout(Instant::now()));
                return Ok(false);
            }
        }

        let (n, source) = self
            .socket
            .recv_from(&mut buf)
            .await
            .map_err(|e| WebRtcClientError::IceParse(format!("recv error: {e}")))?;

        let receive = Receive::new(
            Protocol::Udp,
            source,
            self.local_addr,
            &buf[..n],
        )
        .map_err(|e| WebRtcClientError::IceParse(format!("{e}")))?;

        let _ = self
            .rtc
            .handle_input(Input::Receive(Instant::now(), receive));

        Ok(false)
    }

    pub async fn run(&mut self) -> Result<(), WebRtcClientError> {
        let socket = Arc::clone(&self.socket);
        let mut buf = vec![0u8; 65536];

        loop {
            let timeout_dur = tokio::time::sleep(std::time::Duration::from_secs(1));

            tokio::select! {
                result = socket.recv_from(&mut buf) => {
                    let (n, source) = result.map_err(|e| WebRtcClientError::IceParse(format!("recv error: {e}")))?;
                    let receive = Receive::new(
                        Protocol::Udp,
                        source,
                        self.local_addr,
                        &buf[..n],
                    ).map_err(|e| WebRtcClientError::IceParse(format!("{e}")))?;
                    self.rtc.handle_input(Input::Receive(Instant::now(), receive));
                    self.drain_outputs().await?;
                }

                _ = timeout_dur => {
                    self.rtc.handle_input(Input::Timeout(Instant::now()));
                    self.drain_outputs().await?;
                }
            }
        }
    }

    async fn drain_outputs(&mut self) -> Result<(), WebRtcClientError> {
        loop {
            match self.rtc.poll_output()? {
                Output::Transmit(t) => {
                    self.socket
                        .send_to(&t.contents, t.destination)
                        .await
                        .map_err(|e| WebRtcClientError::IceParse(format!("send error: {e}")))?;
                }
                Output::Event(event) => {
                    self.handle_event(&event);
                }
                Output::Timeout(_) => {
                    return Ok(());
                }
            }
        }
    }

    fn handle_event(&self, event: &Event) {
        match event {
            Event::Connected => {
                log::info!("[webrtc-client] DTLS connected");
            }
            Event::IceConnectionStateChange(state) => {
                log::info!("[webrtc-client] ICE state: {:?}", state);
                if matches!(state, IceConnectionState::Disconnected) {
                    log::info!("[webrtc-client] Peer disconnected");
                }
            }
            Event::ChannelOpen(id, label) => {
                log::info!(
                    "[webrtc-client] Data channel opened: {label} (id={id:?})"
                );
            }
            Event::ChannelData(data) => {
                log::debug!(
                    "[webrtc-client] Channel data: {} bytes on {:?}",
                    data.data.len(),
                    data.id
                );
            }
            Event::ChannelClose(id) => {
                log::info!("[webrtc-client] Data channel closed: {id:?}");
            }
            _ => {}
        }
    }
}
