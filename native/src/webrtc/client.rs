use std::net::SocketAddr;
use std::time::{Duration, Instant};

use str0m::change::SdpOffer;
use str0m::net::{Protocol, Receive};
use str0m::{Candidate, Event, IceConnectionState, Input, Output, Rtc, RtcConfig};
use thiserror::Error;

use crate::webrtc::ice::IceAgent;
use crate::webrtc::signalling::SignalingClient;
use crate::webrtc::socket::{SyncUdpSocket, SyncUdpSocketError};
use schema::devlog::rpc_signalling::server::OfferMessage;

#[derive(Debug, Error)]
pub enum WebRtcClientError {
    #[error("Rtc error: {0}")]
    Rtc(#[from] str0m::error::RtcError),

    #[error("SDP parse error: {0}")]
    SdpParse(String),

    #[error("Signalling error: {0}")]
    Signalling(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Socket error: {0}")]
    Socket(#[from] SyncUdpSocketError),
}

pub struct WebRtcClient {
    rtc: Rtc,
    socket: SyncUdpSocket,
    signalling: SignalingClient,
}

impl WebRtcClient {
    pub async fn connect(
        offer_message: OfferMessage,
        socket: SyncUdpSocket,
        signalling: SignalingClient,
        request_id: String,
        ice_agent: IceAgent,
    ) -> Result<Self, WebRtcClientError> {
        let mut rtc = RtcConfig::new().build(Instant::now());

        let local_addr = socket.local_addr()?;
        let host_candidate = Candidate::host(local_addr, "udp")
            .map_err(|e| WebRtcClientError::Signalling(format!("{e}")))?;
        rtc.add_local_candidate(host_candidate);
        log::info!("[webrtc-client] Added host candidate: {local_addr}");

        ice_agent.gather_candidates(&mut rtc, local_addr).await;

        let offer = SdpOffer::from_sdp_string(&offer_message.sdp)
            .map_err(|e| WebRtcClientError::SdpParse(format!("{e}")))?;

        let answer = rtc
            .sdp_api()
            .accept_offer(offer)
            .map_err(WebRtcClientError::Rtc)?;

        log::info!("[webrtc-client] SDP answer created with all local candidates");

        signalling
            .send_answer(answer.to_sdp_string(), &request_id)
            .await
            .map_err(|e| WebRtcClientError::Signalling(format!("{e}")))?;

        log::info!("[webrtc-client] Answer sent, entering poll loop");

        let mut client = Self {
            rtc,
            socket,
            signalling,
        };

        let mut buf = vec![0u8; 2000];
        loop {
            let timeout = match client.rtc.poll_output()? {
                Output::Timeout(t) => t,
                Output::Transmit(t) => {
                    client.socket.send_to(&t.contents, t.destination).await?;
                    continue;
                }
                Output::Event(e) => {
                    match &e {
                        Event::Connected => {
                            log::info!("[webrtc-client] Connected");
                            return Ok(client);
                        }
                        Event::IceConnectionStateChange(state) => {
                            log::info!("[webrtc-client] ICE state: {:?}", state);
                            if matches!(state, IceConnectionState::Disconnected) {
                                return Err(WebRtcClientError::Signalling(
                                    "Peer disconnected during setup".into(),
                                ));
                            }
                        }
                        _ => {}
                    }
                    continue;
                }
            };

            let duration = timeout.saturating_duration_since(Instant::now());
            if duration.is_zero() {
                client.rtc.handle_input(Input::Timeout(Instant::now()))?;
                continue;
            }

            tokio::select! {
                result = client.socket.recv_any(&mut buf) => {
                    let (n, source) = result?;
                    let receive = Receive::new(Protocol::Udp, source, local_addr, &buf[..n])
                        .map_err(|e| WebRtcClientError::Signalling(format!("{e}")))?;
                    client.rtc.handle_input(Input::Receive(Instant::now(), receive))?;
                }
                _ = tokio::time::sleep(duration) => {
                    log::warn!("[webrtc-client] No data received for 5 seconds during handshake, sending keepalive");
                    client.rtc.handle_input(Input::Timeout(Instant::now()))?;
                }
            }
        }
    }
}
