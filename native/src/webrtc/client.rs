use std::net::SocketAddr;
use std::time::Instant;

use str0m::change::SdpOffer;
use str0m::net::{Protocol, Receive};
use str0m::{Candidate, Event, IceConnectionState, Input, Output, Rtc, RtcConfig};
use thiserror::Error;

use crate::webrtc::signalling::SignalingClient;
use crate::webrtc::socket::SyncUdpSocket;
use schema::devlog::rpc_signalling::server::OfferMessage;

#[derive(Debug, Error)]
pub enum WebRtcClientError {
    #[error("Rtc error: {0}")]
    Rtc(#[from] str0m::error::RtcError),

    #[error("SDP parse error: {0}")]
    SdpParse(String),

    #[error("ICE parse error: {0}")]
    IceParse(String),

    #[error("Signalling error: {0}")]
    Signalling(String),
}

pub struct InboundUdp {
    pub data: Vec<u8>,
    pub source: SocketAddr,
}

pub struct WebRtcClient {
    rtc: Rtc,
    socket: SyncUdpSocket,
    signalling: SignalingClient,
    peer_id: String,
}

impl WebRtcClient {
    pub async fn connect(
        offer_message: OfferMessage,
        gathered_ices: Vec<Candidate>,
        socket: SyncUdpSocket,
        signalling: SignalingClient,
        peer_id: String,
        scopes: Vec<String>,
    ) -> Result<Self, WebRtcClientError> {
        let mut rtc = RtcConfig::new()
            .set_ice_lite(true)
            .build(Instant::now());

        for candidate in gathered_ices {
            rtc.add_remote_candidate(candidate);
        }

        let offer = SdpOffer::from_sdp_string(&offer_message.sdp)
            .map_err(|e| WebRtcClientError::SdpParse(format!("{e}")))?;

        let answer = rtc
            .sdp_api()
            .accept_offer(offer)
            .map_err(WebRtcClientError::Rtc)?;

        signalling
            .send_answer(peer_id.clone(), answer.to_sdp_string(), scopes, peer_id.clone())
            .await
            .map_err(|e| WebRtcClientError::Signalling(format!("{e}")))?;

        let local_addr = socket.local_addr();

        let mut client = Self {
            rtc,
            socket,
            signalling,
            peer_id,
        };

        let mut buf = vec![0u8; 65536];
        loop {
            match client.rtc.poll_output()? {
                Output::Transmit(t) => {
                    client
                        .socket
                        .send_to(&t.contents, t.destination)
                        .await
                        .map_err(|e| WebRtcClientError::IceParse(format!("send error: {e}")))?;
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
                                log::info!("[webrtc-client] Peer disconnected");
                                return Ok(client);
                            }
                        }
                        _ => {}
                    }
                }
                Output::Timeout(deadline) => {
                    let now = Instant::now();
                    if deadline > now {
                        tokio::time::sleep(deadline - now).await;
                    }
                    let _ = client.rtc.handle_input(Input::Timeout(Instant::now()));
                }
            }

            let (n, source) = client
                .socket
                .recv_from(&mut buf)
                .await
                .map_err(|e| WebRtcClientError::IceParse(format!("recv error: {e}")))?;

            let receive = Receive::new(Protocol::Udp, source, local_addr, &buf[..n])
                .map_err(|e| WebRtcClientError::IceParse(format!("{e}")))?;

            let _ = client.rtc.handle_input(Input::Receive(Instant::now(), receive));
        }
    }
}
