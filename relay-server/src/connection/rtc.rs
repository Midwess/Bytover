use std::collections::VecDeque;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

use socket2::{Domain, Socket, Type};
use str0m::channel::{ChannelConfig, ChannelId};
use str0m::net::{Protocol, Receive};
use str0m::{Event, IceConnectionState, Input, Output, Rtc, RtcConfig};
use str0m::change::SdpOffer;
use str0m::Candidate;

use schema::devlog::bitbridge::DataChannel;

const MAX_BUFFER_SIZE: usize = 1024 * 1024 * 5;
const SPEED_METER_WINDOW: Duration = Duration::from_secs(1);

/// How long ICE can stay in `Disconnected` before we treat it as a real disconnect.
/// str0m never emits Failed/Closed ICE states, so we need this timeout.
const ICE_DISCONNECT_TIMEOUT: Duration = Duration::from_secs(5);

/// Maximum number of UDP transmit packets processed in a single `poll_output` call.
/// Capping this prevents the async future state machine from growing too deep
/// during file-transfer bursts, avoiding a stack overflow in the Tokio worker.
const MAX_TRANSMITS_PER_POLL: usize = 8;

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
}

#[derive(Debug)]
pub enum PollOutcome {
    Event(Event),
    Idle(Instant),
    MorePending,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PendingEvent {
    Connected,
    ChannelOpen(ChannelId),
}

pub struct RelayRtcClient {
    rtc: Rtc,
    socket: tokio::net::UdpSocket,
    local_v4_addr: SocketAddr,
    local_v6_addr: SocketAddr,
    buf: Vec<u8>,
    cached_timeout: Instant,
    down_meter: SpeedMeter,
    up_meter: SpeedMeter,

    pending_events: Vec<PendingEvent>,
    connected: bool,
    ice_disconnected_since: Option<Instant>,
}

impl RelayRtcClient {
    pub async fn accept_offer(
        sdp_offer: &str,
        channels: Vec<DataChannel>,
    ) -> Result<(Box<Self>, String), RelayRtcError> {

        let socket = Socket::new(Domain::IPV6, Type::DGRAM, Some(socket2::Protocol::UDP))?;
        socket.set_only_v6(false)?;
        socket.set_nonblocking(true)?;
        socket.bind(&"[::]:0".parse::<SocketAddr>().unwrap().into())?;
        let _ = socket.set_send_buffer_size(MAX_BUFFER_SIZE * 2);
        let socket: std::net::UdpSocket = socket.into();
        let socket = tokio::net::UdpSocket::from_std(socket)?;

        let local_addr = socket.local_addr()?;
        let mut local_v4_addr = local_addr;
        let mut local_v6_addr = local_addr;

        let rtc_config = RtcConfig::default()
            .set_sctp_max_message_size(5 * 1024 * 1024)
            .set_sctp_buffer_size(5 * 1024 * 1024);

        let mut rtc = rtc_config.build(Instant::now());

        let public_ip_v4 = std::env::var("BYTOVER_RELAY_PUBLIC_IP").ok().and_then(|s| s.parse::<std::net::Ipv4Addr>().ok());
        let public_ip_v6 = std::env::var("BYTOVER_RELAY_PUBLIC_IP_V6").ok().and_then(|s| s.parse::<std::net::Ipv6Addr>().ok());

        if let Some(ip4) = public_ip_v4 {
            let addr = SocketAddr::new(ip4.into(), local_addr.port());
            if let Ok(c) = Candidate::host(addr, Protocol::Udp) {
                rtc.add_local_candidate(c);
            }
            local_v4_addr = addr;
        }

        if let Some(ip6) = public_ip_v6 {
            let addr = SocketAddr::new(ip6.into(), local_addr.port());
            if let Ok(c) = Candidate::host(addr, Protocol::Udp) {
                rtc.add_local_candidate(c);
            }
            local_v6_addr = addr;
        }

        if public_ip_v4.is_none() && public_ip_v6.is_none() {
            log::warn!("[relay-rtc] No public IPs configured (BYTOVER_RELAY_PUBLIC_IP or BYTOVER_RELAY_PUBLIC_IP_V6). ICE gathering may fail.");
            let addr = SocketAddr::new(std::net::Ipv4Addr::LOCALHOST.into(), local_addr.port());
            if let Ok(c) = Candidate::host(addr, Protocol::Udp) {
                rtc.add_local_candidate(c);
            }
            local_v4_addr = addr;
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

        let client = Box::new(Self {
            rtc,
            socket,
            local_v4_addr,
            local_v6_addr,
            buf: vec![0u8; 2000],
            cached_timeout: Instant::now(),
            down_meter: SpeedMeter::new(SPEED_METER_WINDOW),
            up_meter: SpeedMeter::new(SPEED_METER_WINDOW),

            pending_events,
            connected: false,
            ice_disconnected_since: None,
        });

        Ok((client, sdp_answer.to_sdp_string()))
    }

    /// Poll the RTC engine for output. Each Transmit is sent immediately to avoid
    /// stacking (which causes stack overflow under high load). Stops at the first Event
    /// or Timeout, returning it for processing.
    pub async fn poll_output(&mut self) -> Result<PollOutcome, RelayRtcError> {
        // Check ICE disconnect timeout before polling
        if let Some(since) = self.ice_disconnected_since {
            if since.elapsed() >= ICE_DISCONNECT_TIMEOUT {
                log::info!("[relay-rtc] ICE disconnected for {:?}, tearing down", ICE_DISCONNECT_TIMEOUT);
                self.rtc.disconnect();
                self.ice_disconnected_since = None;
                return Ok(PollOutcome::Idle(Instant::now() + Duration::from_secs(3600)));
            }
        }

        let mut event = None;
        let mut transmit_count = 0;
        let mut outcome = None;

        loop {
            match self.rtc.poll_output()? {
                Output::Timeout(t) => {
                    self.cached_timeout = t;
                    outcome = Some(PollOutcome::Idle(t));
                    break;
                }
                Output::Transmit(t) => {
                    let dest = to_v6_mapped(t.destination);
                    let len = t.contents.len();

                    if let Err(e) = self.socket.send_to(&t.contents, dest).await {
                        log::warn!("[relay-rtc] Failed to send to {}: {}", dest, e);
                    } else {
                        log::trace!("[relay-rtc] Transmitted {} bytes to {}", len, dest);
                        self.up_meter.record(len as u64);
                    }

                    transmit_count += 1;
                    if transmit_count >= MAX_TRANSMITS_PER_POLL {
                        outcome = Some(PollOutcome::MorePending);
                        break;
                    }
                }
                Output::Event(e) => {
                    event = Some(e);
                    break;
                }
            }
        }

        if let Some(e) = event {
            match &e {
                Event::ChannelData(data) => {
                    log::trace!("[relay-rtc] ChannelData id={:?} len={}", data.id, data.data.len());
                }
                _ => {
                    log::info!("[relay-rtc] Event: {e:?}");
                }
            }
            match &e {
                Event::Connected => {
                    self.pending_events.retain(|p| p != &PendingEvent::Connected);
                }
                Event::ChannelOpen(id, _) => {
                    let target = PendingEvent::ChannelOpen(*id);
                    self.pending_events.retain(|p| p != &target);
                }
                Event::IceConnectionStateChange(state) => {
                    log::info!("[relay-rtc] ICE state changed to {:?}", state);
                    match state {
                        IceConnectionState::Disconnected => {
                            if self.ice_disconnected_since.is_none() {
                                log::info!("[relay-rtc] ICE disconnected, starting {:?} timeout", ICE_DISCONNECT_TIMEOUT);
                                self.ice_disconnected_since = Some(Instant::now());
                            }
                        }
                        IceConnectionState::Connected | IceConnectionState::Completed => {
                            if self.ice_disconnected_since.is_some() {
                                log::info!("[relay-rtc] ICE recovered, clearing disconnect timer");
                            }
                            self.ice_disconnected_since = None;
                        }
                        _ => {}
                    }
                }
                _ => {}
            }

            if !self.connected && self.pending_events.is_empty() {
                self.connected = true;
            }
            return Ok(PollOutcome::Event(e));
        }

        Ok(outcome.unwrap_or(PollOutcome::Idle(self.cached_timeout)))
    }

    pub fn timeout_duration(&self) -> Duration {
        let timeout = self.cached_timeout.saturating_duration_since(Instant::now());
        // Cap wait time when ICE is disconnected so we re-check the timeout sooner
        if self.ice_disconnected_since.is_some() {
            timeout.min(Duration::from_secs(1))
        } else {
            timeout
        }
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
                    self.local_v4_addr
                } else {
                    self.local_v6_addr
                };
                self.down_meter.record(n as u64);
                log::trace!("[relay-rtc] Received {} bytes from {}", n, source);

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
        loop {
            match self.poll_output().await? {
                PollOutcome::Event(e) => return Ok(Some(e)),
                PollOutcome::MorePending => continue,
                PollOutcome::Idle(_) => break,
            }
        }

        if !self.rtc.is_alive() {
            return Ok(None);
        }

        let timeout = self.timeout_duration();
        self.wait_for_input(timeout).await?;
        Ok(None)
    }

    pub fn is_fully_connected(&self) -> bool {
        self.connected
    }

    pub fn is_alive(&self) -> bool {
        self.rtc.is_alive()
    }

    pub fn disconnect(&mut self) {
        self.rtc.disconnect();
    }

    pub fn download_rate_bps(&mut self) -> f64 {
        self.down_meter.rate_bps()
    }

    pub fn upload_rate_bps(&mut self) -> f64 {
        self.up_meter.rate_bps()
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

fn to_v6_mapped(addr: SocketAddr) -> SocketAddr {
    match addr {
        SocketAddr::V4(v4) => SocketAddr::new(v4.ip().to_ipv6_mapped().into(), v4.port()),
        v6 => v6
    }
}
