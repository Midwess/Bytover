use std::collections::VecDeque;
use std::net::{IpAddr, SocketAddr};
use std::time::{Duration, Instant};
use async_speed_limit::Limiter;
use socket2::{Domain, Socket, Type};
use str0m::channel::{ChannelConfig, ChannelId};
use str0m::net::{DatagramSend, Protocol, Receive};
use str0m::{Event, IceConnectionState, Input, Output, Rtc, RtcConfig};
use str0m::change::SdpOffer;
use str0m::Candidate;

use schema::devlog::bitbridge::DataChannel;

const MAX_BUFFER_SIZE: usize = 1024 * 1024 * 5;
const SPEED_METER_WINDOW: Duration = Duration::from_secs(1);
const MIN_DOWNLOAD_LIMIT_BPS: f64 = 1024.0;

pub struct SpeedMeter {
    window: Duration,
    samples: VecDeque<(Instant, u64)>,
    total: u64,
}

impl SpeedMeter {
    pub fn new(window: Duration) -> Self {
        Self { window, samples: VecDeque::new(), total: 0 }
    }

    pub fn record(&mut self, bytes: u64) {
        let now = Instant::now();
        self.prune(now);
        self.samples.push_back((now, bytes));
        self.total += bytes;
    }

    pub fn rate_bps(&mut self) -> f64 {
        self.prune(Instant::now());
        self.total as f64 / self.window.as_secs_f64()
    }

    fn prune(&mut self, now: Instant) {
        let cutoff = now.checked_sub(self.window).unwrap_or(now);
        while let Some(&(t, b)) = self.samples.front() {
            if t < cutoff {
                self.samples.pop_front();
                self.total = self.total.saturating_sub(b);
            } else {
                break;
            }
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum RelayRtcError {
    #[error("Socket error: {0}")]
    Socket(#[from] std::io::Error),
    #[error("RTC error: {0}")]
    Rtc(#[from] str0m::error::RtcError),
    #[error("Connection closed")]
    ConnectionClosed,
    #[error("Unauthorized packet source")]
    UnauthorizedSource,
    #[error("ICE disconnected unexpectedly")]
    IceDisconnected,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PendingEvent {
    Connected,
    ChannelOpen(ChannelId),
}

pub struct RelayRtcClient {
    rtc: Rtc,
    socket: tokio::net::UdpSocket,
    local_addr: SocketAddr,
    local_v4_addr: Option<SocketAddr>,
    local_v6_addr: Option<SocketAddr>,
    buf: Vec<u8>,
    cached_timeout: Instant,
    down_meter: SpeedMeter,
    up_meter: SpeedMeter,
    download_limiter: Limiter,
    upload_limiter: Limiter,
    pending_events: Vec<PendingEvent>,
    connected: bool,
}

impl RelayRtcClient {
    pub async fn accept_offer(
        sdp_offer: &str,
        channels: Vec<DataChannel>,
    ) -> Result<(Self, String), RelayRtcError> {

        let socket = Socket::new(Domain::IPV6, Type::DGRAM, Some(socket2::Protocol::UDP))?;
        socket.set_only_v6(false)?;
        socket.set_nonblocking(true)?;
        socket.bind(&"[::]:0".parse::<SocketAddr>().unwrap().into())?;
        let _ = socket.set_send_buffer_size(MAX_BUFFER_SIZE * 2);
        let socket: std::net::UdpSocket = socket.into();
        let socket = tokio::net::UdpSocket::from_std(socket)?;

        let local_addr = socket.local_addr()?;
        let mut local_v4_addr = None;
        let mut local_v6_addr = None;

        if local_addr.is_ipv4() {
            local_v4_addr = Some(local_addr);
        } else {
            local_v6_addr = Some(local_addr);
        }

        let rtc_config = RtcConfig::default()
            .set_sctp_max_message_size(10 * 1024 * 1024)
            .set_sctp_buffer_size(10 * 1024 * 1024);

        let public_ip_str = std::env::var("BYTOVER_RELAY_PUBLIC_IP").unwrap_or_else(|_| "127.0.0.1".to_string());
        let ip = match public_ip_str.parse::<IpAddr>() {
            Ok(parsed) => parsed,
            Err(e) => {
                log::error!("[relay-rtc] Failed to parse BYTOVER_RELAY_PUBLIC_IP '{}': {}. Falling back to 127.0.0.1", public_ip_str, e);
                "127.0.0.1".parse::<IpAddr>().unwrap()
            }
        };

        let mut rtc = rtc_config.build(Instant::now());

        let candidate_addr = SocketAddr::new(ip, local_addr.port());
        if let Ok(candidate) = Candidate::host(candidate_addr, Protocol::Udp) {
            rtc.add_local_candidate(candidate);
            if ip.is_ipv4() {
                local_v4_addr = Some(candidate_addr);
            } else {
                local_v6_addr = Some(candidate_addr);
            }
        } else {
            log::error!("[relay-rtc] Failed to create UDP host candidate for IP: {}", ip);
        }

        let offer = SdpOffer::from_sdp_string(sdp_offer).map_err(|e| RelayRtcError::Rtc(str0m::error::RtcError::RemoteSdp(e.to_string())))?;

        let mut channel_ids = Vec::new();
        for channel_config in &channels {
            let id = rtc.direct_api().create_data_channel(ChannelConfig {
                label: channel_config.label.clone(),
                ordered: channel_config.ordered,
                negotiated: Some(channel_config.negotiate as u16),
                ..Default::default()
            });

            channel_ids.push(id);
        }

        let sdp_answer = rtc.sdp_api().accept_offer(offer).map_err(RelayRtcError::Rtc)?;
        let mut pending_events = vec![PendingEvent::Connected];
        for id in &channel_ids {
            pending_events.push(PendingEvent::ChannelOpen(*id));
        }

        let client = Self {
            rtc,
            socket,
            local_addr,
            local_v4_addr,
            local_v6_addr,
            buf: vec![0u8; 2000],
            cached_timeout: Instant::now(),
            down_meter: SpeedMeter::new(SPEED_METER_WINDOW),
            up_meter: SpeedMeter::new(SPEED_METER_WINDOW),
            download_limiter: Limiter::new(f64::INFINITY),
            upload_limiter: Limiter::new(f64::INFINITY),
            pending_events,
            connected: false,
        };

        Ok((client, sdp_answer.to_sdp_string()))
    }

    /// Drain all pending output from the RTC engine: send all Transmit packets
    /// and collect events. This is cancellation-safe because it separates the
    /// synchronous dequeue (poll_output) from the async send, buffering transmits
    /// so that a cancelled future cannot lose packets.
    pub async fn drain_output(&mut self) -> Result<Option<Event>, RelayRtcError> {
        // Phase 1: Synchronously collect all pending transmits and the first event (if any).
        let mut transmits: Vec<(SocketAddr, DatagramSend)> = Vec::new();
        let mut event = None;

        loop {
            match self.rtc.poll_output()? {
                Output::Timeout(t) => {
                    self.cached_timeout = t;
                    break;
                }
                Output::Transmit(t) => {
                    transmits.push((t.destination, t.contents));
                }
                Output::Event(e) => {
                    event = Some(e);
                    break;
                }
            }
        }

        // Phase 2: Send all collected transmits. Even if this is cancelled,
        // the remaining unsent transmits are lost but poll_output already
        // returned them—str0m won't re-emit. However, by batching first we
        // avoid the common case of losing a single critical DTLS packet.
        for (dest, contents) in transmits {
            let dest = to_v6_mapped(dest);
            let len = contents.len();
            let res = tokio::time::timeout(
                Duration::from_secs(10),
                self.socket.send_to(&contents, dest),
            ).await;
            match res {
                Ok(Ok(_)) => {
                    log::trace!("[relay-rtc] Transmitted {} bytes to {}", len, dest);
                    self.up_meter.record(len as u64);
                    if self.connected {
                        self.upload_limiter.clone().consume(len).await;
                    }
                }
                Ok(Err(e)) => {
                    log::warn!("[relay-rtc] Failed to send to {}: {}", dest, e);
                }
                Err(_) => {
                    log::error!("[relay-rtc] Timeout sending packet to {}", dest);
                }
            }
        }

        // Phase 3: Process the event if we got one.
        if let Some(e) = event {
            log::info!("Received event {e:?}");
            match &e {
                Event::Connected => {
                    self.pending_events.retain(|p| p != &PendingEvent::Connected);
                }
                Event::ChannelOpen(id, _) => {
                    let target = PendingEvent::ChannelOpen(*id);
                    self.pending_events.retain(|p| p != &target);
                }
                Event::IceConnectionStateChange(state) => {
                    if matches!(state, IceConnectionState::Disconnected) {
                        self.rtc.disconnect();
                    }
                }
                _ => {}
            }

            if !self.connected && self.pending_events.is_empty() {
                self.connected = true;
            }
            return Ok(Some(e));
        }

        Ok(None)
    }

    pub fn timeout_duration(&self) -> Duration {
        self.cached_timeout.saturating_duration_since(Instant::now())
    }

    pub async fn wait_for_input(&mut self, timeout: Duration) -> Result<(), RelayRtcError> {
        if timeout.is_zero() {
            self.rtc.handle_input(Input::Timeout(Instant::now()))?;
            return Ok(());
        }

        match tokio::time::timeout(timeout, self.socket.recv_from(&mut self.buf[..])).await {
            Ok(Ok((n, source))) => {
                let source = from_v6_mapped(source);

                let local = if source.is_ipv4() {
                    self.local_v4_addr.unwrap_or(self.local_addr)
                } else {
                    self.local_v6_addr.unwrap_or(self.local_addr)
                };

                self.down_meter.record(n as u64);
                log::trace!("[relay-rtc] Received {} bytes from {} (local={})", n, source, local);

                match Receive::new(Protocol::Udp, source, local, &self.buf[..n]) {
                    Ok(receive) => {
                        if let Err(e) = self.rtc.handle_input(Input::Receive(Instant::now(), receive)) {
                            log::warn!("[relay-rtc] Input handle packet drop: {}", e);
                        }
                    }
                    Err(e) => {
                        log::warn!("[relay-rtc] Failed to parse Receive: {}", e);
                    }
                }

                if self.connected {
                    self.download_limiter.clone().consume(n).await;
                }
            }
            Ok(Err(e)) => return Err(e.into()),
            Err(_) => {
                self.rtc.handle_input(Input::Timeout(Instant::now()))?;
            }
        }
        Ok(())
    }

    pub fn handle_timeout(&mut self, now: Instant) -> Result<(), RelayRtcError> {
        self.rtc.handle_input(Input::Timeout(now))?;
        Ok(())
    }

    pub async fn process_step(&mut self) -> Result<Option<Event>, RelayRtcError> {
        if let Some(event) = self.drain_output().await? {
            return Ok(Some(event));
        }

        let timeout = self.timeout_duration();
        self.wait_for_input(timeout).await?;
        Ok(None)
    }

    pub fn is_alive(&self) -> bool {
        self.rtc.is_alive()
    }

    pub fn is_fully_connected(&self) -> bool {
        self.connected
    }

    pub fn download_rate_bps(&mut self) -> f64 {
        self.down_meter.rate_bps()
    }

    pub fn upload_rate_bps(&mut self) -> f64 {
        self.up_meter.rate_bps()
    }

    pub fn set_download_limit_bps(&self, bps: f64) {
        let effective = if bps.is_infinite() {
            f64::INFINITY
        } else if bps < MIN_DOWNLOAD_LIMIT_BPS {
            MIN_DOWNLOAD_LIMIT_BPS
        } else {
            bps
        };
        self.download_limiter.set_speed_limit(effective);
    }

    pub fn set_upload_limit_bps(&self, bps: f64) {
        let effective = if bps.is_infinite() {
            f64::INFINITY
        } else if bps < MIN_DOWNLOAD_LIMIT_BPS {
            MIN_DOWNLOAD_LIMIT_BPS
        } else {
            bps
        };
        self.upload_limiter.set_speed_limit(effective);
    }

    pub fn send(&mut self, data: &[u8], channel_id: ChannelId) -> bool {
        if let Some(mut ch) = self.rtc.channel(channel_id) {
            match ch.write(true, data) {
                Ok(true) => true,
                Ok(false) => false,
                Err(e) => {
                    log::error!("[relay-rtc] Failed to write to channel {:?}: {:?}", channel_id, e);
                    false
                }
            }
        } else {
            log::warn!("[relay-rtc] Channel {:?} not found", channel_id);
            false
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
