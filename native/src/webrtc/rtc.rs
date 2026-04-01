use std::net::SocketAddr;
use std::time::{Duration, Instant};
use socket2::{Domain, Socket, Type};
use str0m::channel::{ChannelConfig, ChannelId};
use str0m::net::{Protocol, Receive};
use str0m::{Event, IceConnectionState, Input, Output, Rtc};

use schema::devlog::rpc_signalling::server::OfferMessage;

use crate::webrtc::client::{MAX_BUFFER_SIZE, WebRtcClientError};
use crate::webrtc::ice::IceAgent;
use crate::webrtc::signalling::SignallingSender;

pub const RELIABLE_STREAM_ID: u16 = 1;
pub const UNRELIABLE_STREAM_ID: u16 = 2;
pub const UNORDERED_MSG_STREAM_ID: u16 = 3;
pub const ORDERED_MSG_STREAM_ID: u16 = 4;

#[derive(Debug, Clone, Copy)]
pub struct ChannelIds {
    pub reliable: ChannelId,
    pub unreliable: ChannelId,
    pub unordered_msg: ChannelId,
    pub ordered_msg: ChannelId
}

pub struct RtcClient {
    rtc: Rtc,
    socket: tokio::net::UdpSocket,
    local_addr: SocketAddr,
    local_v4_addr: Option<SocketAddr>,
    local_v6_addr: Option<SocketAddr>,
    buf: Vec<u8>,
    cached_timeout: Instant,
    channel_ids: ChannelIds
}

impl RtcClient {
    pub async fn connect(
        signalling_id: &str,
        offer_message: OfferMessage,
        signalling: SignallingSender,
        request_id: &str
    ) -> Result<Self, WebRtcClientError> {
        let config = match signalling.fetch_relay_config(signalling_id).await {
            Ok(c) => c,
            Err(e) => {
                log::warn!(
                    "[rtc-client] Failed to fetch relay config ({}), proceeding without TURN relay",
                    e
                );
                schema::devlog::rpc_signalling::server::IceConfig::default()
            }
        };

        let socket = Socket::new(Domain::IPV6, Type::DGRAM, Some(socket2::Protocol::UDP))?;
        socket.set_only_v6(false)?;
        socket.set_nonblocking(true)?;
        socket.bind(&"[::]:0".parse::<SocketAddr>().unwrap().into())?;
        let _ = socket.set_send_buffer_size(MAX_BUFFER_SIZE * 2);
        let socket: std::net::UdpSocket = socket.into();
        let socket = tokio::net::UdpSocket::from_std(socket)?;

        let local_addr = socket.local_addr()?;

        let (candidates, _relay_client) = IceAgent::gather_candidates(&socket, &config)
            .await
            .map_err(|e| WebRtcClientError::Signalling(format!("{e}")))?;

        let mut rtc = Rtc::new(Instant::now());
        let mut local_v4_addr = None;
        let mut local_v6_addr = None;
        for candidate in candidates {
            if candidate.addr().is_ipv4() && local_v4_addr.is_none() {
                local_v4_addr = Some(candidate.addr());
            } else if candidate.addr().is_ipv6() && local_v6_addr.is_none() {
                local_v6_addr = Some(candidate.addr());
            }
            rtc.add_local_candidate(candidate);
        }

        let offer_sdp = IceAgent::resolve_remote_candidates(&offer_message.sdp);
        log::info!("Received offer sdp: {offer_sdp}");
        let offer = str0m::change::SdpOffer::from_sdp_string(&offer_sdp).map_err(|e| WebRtcClientError::SdpParse(format!("{e}")))?;

        let reliable_id = rtc.direct_api().create_data_channel(ChannelConfig {
            label: "reliable".to_string(),
            ordered: false,
            negotiated: Some(RELIABLE_STREAM_ID),
            ..Default::default()
        });
        let unreliable_id = rtc.direct_api().create_data_channel(ChannelConfig {
            label: "unreliable".to_string(),
            ordered: false,
            reliability: str0m::channel::Reliability::MaxRetransmits { retransmits: 0 },
            negotiated: Some(UNRELIABLE_STREAM_ID),
            ..Default::default()
        });
        let unordered_msg_id = rtc.direct_api().create_data_channel(ChannelConfig {
            label: "unordered_msg".to_string(),
            ordered: false,
            negotiated: Some(UNORDERED_MSG_STREAM_ID),
            ..Default::default()
        });
        let ordered_msg_id = rtc.direct_api().create_data_channel(ChannelConfig {
            label: "ordered_msg".to_string(),
            ordered: true,
            negotiated: Some(ORDERED_MSG_STREAM_ID),
            ..Default::default()
        });
        let channel_ids = ChannelIds {
            reliable: reliable_id,
            unreliable: unreliable_id,
            unordered_msg: unordered_msg_id,
            ordered_msg: ordered_msg_id
        };

        let answer = rtc.sdp_api().accept_offer(offer).map_err(WebRtcClientError::Rtc)?;

        signalling
            .send_answer(answer.to_sdp_string(), request_id)
            .await
            .map_err(|e| WebRtcClientError::Signalling(format!("{e}")))?;

        let mut client = Self {
            rtc,
            socket,
            local_addr,
            local_v4_addr,
            local_v6_addr,
            buf: vec![0u8; 2000],
            cached_timeout: Instant::now(),
            channel_ids
        };

        let mut connected = false;
        loop {
            if let Some(event) = client.poll_event().await? {
                match event {
                    Event::Connected => {
                        log::info!("[rtc-client] DTLS Connected");
                        connected = true;
                    }
                    Event::IceConnectionStateChange(state) => {
                        log::info!("[rtc-client] ICE state: {:?}", state);
                        if matches!(state, IceConnectionState::Disconnected) {
                            return Err(WebRtcClientError::Signalling("Peer disconnected during setup".into()));
                        }
                    }
                    _ => {}
                }
            }

            if connected {
                let ready = [
                    channel_ids.reliable,
                    channel_ids.unreliable,
                    channel_ids.unordered_msg,
                    channel_ids.ordered_msg
                ]
                .iter()
                .all(|&cid| client.rtc.channel(cid).is_some());

                if ready {
                    log::info!("[rtc-client] Connected, negotiated channels ready");
                    return Ok(client);
                }
            }

            if !client.rtc.is_alive() {
                return Err(WebRtcClientError::Signalling("RTC connection closed".into()));
            }

            let timeout = client.timeout_duration();
            client.wait_for_input(timeout).await?;
        }
    }

    pub async fn poll_event(&mut self) -> Result<Option<Event>, WebRtcClientError> {
        loop {
            match self.rtc.poll_output()? {
                Output::Timeout(t) => {
                    self.cached_timeout = t;
                    return Ok(None);
                }
                Output::Transmit(t) => {
                    let dest = to_v6_mapped(t.destination);
                    let res = tokio::time::timeout(Duration::from_secs(10), self.socket.send_to(&t.contents, dest)).await;

                    match res {
                        Ok(Err(e)) => {
                            log::warn!("[rtc-client] Failed to send to {}: {}", dest, e);
                            return Err(WebRtcClientError::Signalling(format!("Failed to send packet: {:?}", e)));
                        }
                        Err(_) => {
                            log::error!("[rtc-client] Timeout sending packet to {} after 10s", dest);
                            return Err(WebRtcClientError::Signalling("Send packet timed out".to_string()));
                        }
                        _ => {}
                    }
                }
                Output::Event(e) => {
                    if let Event::IceConnectionStateChange(state) = e {
                        if matches!(state, IceConnectionState::Disconnected) {
                            self.rtc.disconnect();
                        }
                    }
                    return Ok(Some(e));
                }
            }
        }
    }

    pub fn timeout_duration(&self) -> Duration {
        self.cached_timeout.saturating_duration_since(Instant::now())
    }

    pub async fn wait_for_input(&mut self, timeout: Duration) -> Result<(), WebRtcClientError> {
        if timeout.is_zero() {
            self.rtc.handle_input(Input::Timeout(Instant::now()))?;
            return Ok(());
        }

        match tokio::time::timeout(timeout, self.socket.recv_from(&mut self.buf[..])).await {
            Ok(Ok((n, mut source))) => {
                source = from_v6_mapped(source);
                let local = if source.is_ipv4() {
                    self.local_v4_addr.unwrap_or(self.local_addr)
                } else {
                    self.local_v6_addr.unwrap_or(self.local_addr)
                };
                match Receive::new(Protocol::Udp, source, local, &self.buf[..n]) {
                    Ok(receive) => {
                        if let Err(e) = self.rtc.handle_input(Input::Receive(Instant::now(), receive)) {
                            log::warn!("[rtc-client] Input handle packet drop: {}", e);
                        }
                    }
                    Err(e) => {
                        log::warn!("[rtc-client] Failed to parse Receive: {}", e);
                    }
                }
            }
            Ok(Err(e)) => return Err(e.into()),
            Err(_) => {
                self.rtc.handle_input(Input::Timeout(Instant::now()))?;
            }
        }
        Ok(())
    }

    pub fn send(&mut self, data: &[u8], channel_id: ChannelId) -> bool {
        if let Some(mut ch) = self.rtc.channel(channel_id) {
            match ch.write(true, data) {
                Ok(true) => true,
                Ok(false) => false, 
                Err(e) => {
                    log::error!("[rtc-client] Failed to write to channel {:?}: {:?}", channel_id, e);
                    false
                }
            }
        } else {
            log::warn!("[rtc-client] Channel {:?} not found", channel_id);
            false
        }
    }

    pub fn set_buffered_amount_low_threshold(&mut self, channel_id: ChannelId, threshold: usize) {
        if let Some(mut ch) = self.rtc.channel(channel_id) {
            ch.set_buffered_amount_low_threshold(threshold);
        }
    }

    pub fn channel_ids(&self) -> &ChannelIds {
        &self.channel_ids
    }

    pub fn is_alive(&self) -> bool {
        self.rtc.is_alive()
    }
}

fn to_v6_mapped(addr: SocketAddr) -> SocketAddr {
    match addr {
        SocketAddr::V4(v4) => SocketAddr::new(v4.ip().to_ipv6_mapped().into(), v4.port()),
        v6 => v6
    }
}

fn from_v6_mapped(addr: SocketAddr) -> SocketAddr {
    match addr {
        SocketAddr::V6(v6) => {
            let octets = v6.ip().octets();
            if octets[0..12] == [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xff, 0xff] {
                let v4 = std::net::Ipv4Addr::new(octets[12], octets[13], octets[14], octets[15]);
                SocketAddr::new(v4.into(), v6.port())
            } else {
                addr
            }
        }
        _ => addr
    }
}
