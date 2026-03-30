use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

use futures::channel::mpsc;
use futures_util::stream::StreamExt;
use socket2::{Domain, Socket, Type};
use str0m::channel::{ChannelConfig, ChannelId};
use str0m::net::{Protocol, Receive};
use str0m::{Event, IceConnectionState, Input, Output, Rtc};
use tokio::net::UdpSocket;

use schema::devlog::rpc_signalling::server::OfferMessage;

use crate::webrtc::client::WebRtcClientError;
use crate::webrtc::ice::IceAgent;
use crate::webrtc::signalling::SignalingClient;

const TOTAL_CHANNELS: usize = 4;

const fn channel_id(raw: usize) -> ChannelId {
    unsafe { std::mem::transmute(raw) }
}

pub const RELIABLE_DATA_CHANNEL_ID: ChannelId = channel_id(1);
pub const UNRELIABLE_DATA_CHANNEL_ID: ChannelId = channel_id(2);
pub const UNORDERED_MSG_CHANNEL_ID: ChannelId = channel_id(3);
pub const ORDERED_MSG_CHANNEL_ID: ChannelId = channel_id(4);

pub enum RtcEvent {
    Connected,
    ChannelOpen(ChannelId, String),
    ChannelData { id: ChannelId, data: Vec<u8> },
    IceConnectionStateChange(IceConnectionState),
    Closed,
}

pub struct ChannelSenders {
    pub ordered_msg_tx: mpsc::Sender<Box<[u8]>>,
    pub unordered_msg_tx: mpsc::Sender<Box<[u8]>>,
    pub reliable_data_tx: mpsc::Sender<Box<[u8]>>,
    pub unreliable_data_tx: mpsc::Sender<Box<[u8]>>,
}

pub struct RtcClient {
    rtc: Rtc,
    socket: UdpSocket,
    local_addr: SocketAddr,
    local_v4_addr: Option<SocketAddr>,
    local_v6_addr: Option<SocketAddr>,
    buf: Vec<u8>,
    ordered_msg_rx: Option<mpsc::Receiver<Box<[u8]>>>,
    unordered_msg_rx: Option<mpsc::Receiver<Box<[u8]>>>,
    reliable_data_rx: Option<mpsc::Receiver<Box<[u8]>>>,
    unreliable_data_rx: Option<mpsc::Receiver<Box<[u8]>>>,
}

impl RtcClient {
    pub async fn connect(
        signalling_id: &str,
        offer_message: OfferMessage,
        signalling: &SignalingClient,
        request_id: &str,
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
        let std_socket: std::net::UdpSocket = socket.into();
        let socket = UdpSocket::from_std(std_socket)?;
        let socket = Arc::new(socket);

        let local_addr = socket.local_addr()?;

        let (candidates, _relay_client) = IceAgent::gather_candidates(socket.clone(), &config)
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

        let offer_sdp = IceAgent::resolve_remote_candidates(&offer_message.sdp).await;
        log::info!("Received offer sdp: {offer_sdp}");
        let offer = str0m::change::SdpOffer::from_sdp_string(&offer_sdp)
            .map_err(|e| WebRtcClientError::SdpParse(format!("{e}")))?;

        let answer = rtc.sdp_api().accept_offer(offer).map_err(WebRtcClientError::Rtc)?;

        let mut api = rtc.sdp_api();
        api.add_channel_with_config(ChannelConfig {
            label: "reliable".to_string(),
            ordered: true,
            negotiated: Some(1),
            ..Default::default()
        });
        api.add_channel_with_config(ChannelConfig {
            label: "unreliable".to_string(),
            ordered: false,
            negotiated: Some(2),
            ..Default::default()
        });
        api.add_channel_with_config(ChannelConfig {
            label: "unordered_msg".to_string(),
            ordered: false,
            negotiated: Some(3),
            ..Default::default()
        });
        api.add_channel_with_config(ChannelConfig {
            label: "ordered_msg".to_string(),
            ordered: true,
            negotiated: Some(4),
            ..Default::default()
        });

        signalling
            .send_answer(answer.to_sdp_string(), request_id)
            .await
            .map_err(|e| WebRtcClientError::Signalling(format!("{e}")))?;

        log::info!("[rtc-client] Answer sent, waiting for connection and all channels");

        let mut client = Self {
            rtc,
            socket: Arc::try_unwrap(socket).expect("socket Arc should have single owner after gather"),
            local_addr,
            local_v4_addr,
            local_v6_addr,
            buf: vec![0u8; 2000],
            ordered_msg_rx: None,
            unordered_msg_rx: None,
            reliable_data_rx: None,
            unreliable_data_rx: None,
        };

        let mut channels_opened: usize = 0;
        let mut is_connected = false;

        loop {
            match client.poll_loop().await? {
                RtcEvent::Connected => {
                    log::info!("[rtc-client] Connected");
                    is_connected = true;
                }
                RtcEvent::ChannelOpen(_, ref label) => {
                    channels_opened += 1;
                    log::info!("[rtc-client] Channel {} opened (label: {})", channels_opened, label);
                    if is_connected && channels_opened >= TOTAL_CHANNELS {
                        log::info!("[rtc-client] All channels open, ready");
                        return Ok(client);
                    }
                }
                RtcEvent::IceConnectionStateChange(state) => {
                    log::info!("[rtc-client] ICE state: {:?}", state);
                    if matches!(state, IceConnectionState::Disconnected) {
                        return Err(WebRtcClientError::Signalling("Peer disconnected during setup".into()));
                    }
                }
                _ => {}
            }
        }
    }

    pub fn setup_channels(&mut self) -> ChannelSenders {
        let (ordered_msg_tx, ordered_msg_rx) = mpsc::channel::<Box<[u8]>>(64);
        let (unordered_msg_tx, unordered_msg_rx) = mpsc::channel::<Box<[u8]>>(64);
        let (reliable_data_tx, reliable_data_rx) = mpsc::channel::<Box<[u8]>>(64);
        let (unreliable_data_tx, unreliable_data_rx) = mpsc::channel::<Box<[u8]>>(64);

        self.ordered_msg_rx = Some(ordered_msg_rx);
        self.unordered_msg_rx = Some(unordered_msg_rx);
        self.reliable_data_rx = Some(reliable_data_rx);
        self.unreliable_data_rx = Some(unreliable_data_rx);

        ChannelSenders {
            ordered_msg_tx,
            unordered_msg_tx,
            reliable_data_tx,
            unreliable_data_tx,
        }
    }

    pub async fn poll_loop(&mut self) -> Result<RtcEvent, WebRtcClientError> {
        loop {
            if !self.rtc.is_alive() {
                return Ok(RtcEvent::Closed);
            }

            let timeout: Instant = {
                loop {
                    match self.rtc.poll_output()? {
                        Output::Timeout(t) => break t,
                        Output::Transmit(t) => {
                            let dest = to_v6_mapped(t.destination);
                            if let Err(e) = self.socket.send_to(&t.contents, dest).await {
                                log::warn!("[rtc-client] Failed to send to {}: {}", dest, e);
                            }
                        }
                        Output::Event(e) => match e {
                            Event::Connected => return Ok(RtcEvent::Connected),
                            Event::ChannelOpen(id, label) => return Ok(RtcEvent::ChannelOpen(id, label)),
                            Event::ChannelData(data) => {
                                return Ok(RtcEvent::ChannelData { id: data.id, data: data.data });
                            }
                            Event::IceConnectionStateChange(state) => {
                                if matches!(state, IceConnectionState::Disconnected) {
                                    self.rtc.disconnect();
                                }
                                return Ok(RtcEvent::IceConnectionStateChange(state));
                            }
                            _ => {}
                        },
                    }
                }
            };

            let duration = timeout.saturating_duration_since(Instant::now());
            if duration.is_zero() {
                self.rtc.handle_input(Input::Timeout(Instant::now()))?;
                continue;
            }

            let Self {
                rtc, socket, buf, local_addr, local_v4_addr, local_v6_addr,
                ordered_msg_rx, unordered_msg_rx, reliable_data_rx, unreliable_data_rx,
            } = self;

            tokio::select! {
                res = socket.recv_from(&mut buf[..]) => {
                    if let Ok((n, mut source)) = res {
                        source = from_v6_mapped(source);
                        let local = if source.is_ipv4() {
                            local_v4_addr.unwrap_or(*local_addr)
                        } else {
                            local_v6_addr.unwrap_or(*local_addr)
                        };
                        match Receive::new(Protocol::Udp, source, local, &buf[..n]) {
                            Ok(receive) => {
                                if let Err(e) = rtc.handle_input(Input::Receive(Instant::now(), receive)) {
                                    log::trace!("[rtc-client] Input handle packet drop: {}", e);
                                }
                            }
                            Err(e) => {
                                log::trace!("[rtc-client] Failed to parse Receive: {}", e);
                            }
                        }
                    }
                }
                _ = tokio::time::sleep(duration) => {
                    rtc.handle_input(Input::Timeout(Instant::now()))?;
                }
                Some(data) = async { ordered_msg_rx.as_mut()?.next().await } => {
                    if let Some(mut ch) = rtc.channel(ORDERED_MSG_CHANNEL_ID) {
                        ch.write(true, &data).ok();
                    }
                }
                Some(data) = async { unordered_msg_rx.as_mut()?.next().await } => {
                    if let Some(mut ch) = rtc.channel(UNORDERED_MSG_CHANNEL_ID) {
                        ch.write(true, &data).ok();
                    }
                }
                Some(data) = async { reliable_data_rx.as_mut()?.next().await } => {
                    if let Some(mut ch) = rtc.channel(RELIABLE_DATA_CHANNEL_ID) {
                        ch.write(true, &data).ok();
                    }
                }
                Some(data) = async { unreliable_data_rx.as_mut()?.next().await } => {
                    if let Some(mut ch) = rtc.channel(UNRELIABLE_DATA_CHANNEL_ID) {
                        ch.write(true, &data).ok();
                    }
                }
            }
        }
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
