use std::collections::VecDeque;
use std::net::{Ipv4Addr, SocketAddr};
use std::time::{Duration, Instant};

use async_speed_limit::Limiter;
use socket2::{Domain, Socket, Type};
use str0m::change::SdpOffer;
use str0m::channel::{ChannelConfig, ChannelId};
use str0m::net::{Protocol, Receive};
use str0m::{Candidate, Event, IceConnectionState, Input, Output, Rtc, RtcConfig};

use schema::devlog::bitbridge::DataChannel;

const MAX_BUFFER_SIZE: usize = 1024 * 1024 * 5;
const SPEED_METER_WINDOW: Duration = Duration::from_secs(1);
const QUEUE_SOFT_LIMIT_BYTES: usize = 5 * 1024 * 1024;
const QUEUE_MEASURE_START_BYTES: usize = QUEUE_SOFT_LIMIT_BYTES / 2;
const QUEUE_MATCH_TARGET_BYTES: usize = QUEUE_SOFT_LIMIT_BYTES * 5 / 4;
const QUEUE_HARD_LIMIT_BYTES: usize = QUEUE_SOFT_LIMIT_BYTES * 2;
const MIN_UPLOAD_SPEED_BPS: f64 = 10.0 * 1024.0;

/// How long ICE can stay in `Disconnected` before we treat it as a real disconnect.
/// str0m never emits Failed/Closed ICE states, so we need this timeout.
const ICE_DISCONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// Maximum number of consecutive `MorePending` cycles processed in a single
/// `process_step` call before yielding back to the runtime.
const MAX_PENDING_SPINS_PER_STEP: usize = 8;

pub struct SpeedMeter {
    window: Duration,
    samples: VecDeque<(Instant, u64)>,
    total: u64,
}

impl SpeedMeter {
    pub fn new(window: Duration) -> Self {
        Self {
            window,
            samples: VecDeque::new(),
            total: 0,
        }
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

struct RateLimitedUdpSocket {
    socket: tokio::net::UdpSocket,
    upload_limiter: Limiter,
    upload_speed_limit_bps: f64,
    down_meter: SpeedMeter,
    up_meter: SpeedMeter,
}

impl RateLimitedUdpSocket {
    fn new(socket: tokio::net::UdpSocket) -> Self {
        Self {
            socket,
            upload_limiter: Limiter::new(f64::INFINITY),
            upload_speed_limit_bps: f64::INFINITY,
            down_meter: SpeedMeter::new(SPEED_METER_WINDOW),
            up_meter: SpeedMeter::new(SPEED_METER_WINDOW),
        }
    }

    async fn send_to(&mut self, buf: &[u8], dest: SocketAddr) -> Result<usize, std::io::Error> {
        self.upload_limiter.consume(buf.len()).await;
        match self.socket.send_to(buf, dest).await {
            Ok(sent) => {
                if sent < buf.len() {
                    self.upload_limiter.unconsume(buf.len() - sent);
                }
                self.up_meter.record(sent as u64);
                Ok(sent)
            }
            Err(err) => {
                self.upload_limiter.unconsume(buf.len());
                Err(err)
            }
        }
    }

    async fn recv_from(&mut self, buf: &mut [u8]) -> Result<(usize, SocketAddr), std::io::Error> {
        let (received, source) = self.socket.recv_from(buf).await?;
        self.down_meter.record(received as u64);
        Ok((received, source))
    }

    fn set_upload_speed_limit(&mut self, speed_limit_bps: f64) {
        if (self.upload_speed_limit_bps.is_infinite() && speed_limit_bps.is_infinite())
            || (!self.upload_speed_limit_bps.is_infinite()
                && !speed_limit_bps.is_infinite()
                && (self.upload_speed_limit_bps - speed_limit_bps).abs() < 1.0)
        {
            return;
        }

        self.upload_limiter.set_speed_limit(speed_limit_bps);
        self.upload_speed_limit_bps = speed_limit_bps;
    }

    fn download_rate_bps(&mut self) -> f64 {
        self.down_meter.rate_bps()
    }

    fn upload_rate_bps(&mut self) -> f64 {
        self.up_meter.rate_bps()
    }
}

#[derive(Default)]
struct UploadLimitState {
    measured_upload_bps: Option<f64>,
    limit_active_logged: bool,
    hard_cap_logged: bool,
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
    Idle,
    MorePending,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PendingEvent {
    Connected,
    ChannelOpen(ChannelId),
}

pub struct RelayRtcClient {
    rtc: Rtc,
    socket: RateLimitedUdpSocket,
    local_v4_addr: SocketAddr,
    local_v6_addr: SocketAddr,
    buf: Vec<u8>,
    cached_timeout: Instant,
    pending_events: Vec<PendingEvent>,
    connected: bool,
    ice_disconnected_since: Option<Instant>,
    upload_limit: UploadLimitState,
}

impl RelayRtcClient {
    pub async fn accept_offer(
        sdp_offer: &str,
        channels: Vec<DataChannel>,
        public_ipv4: Ipv4Addr,
    ) -> Result<(Box<Self>, String), RelayRtcError> {
        let socket = Socket::new(Domain::IPV6, Type::DGRAM, Some(socket2::Protocol::UDP))?;
        socket.set_only_v6(false)?;
        socket.set_nonblocking(true)?;
        socket.bind(&"[::]:0".parse::<SocketAddr>().unwrap().into())?;
        let _ = socket.set_send_buffer_size(MAX_BUFFER_SIZE);
        let socket: std::net::UdpSocket = socket.into();
        let socket = tokio::net::UdpSocket::from_std(socket)?;

        let local_addr = socket.local_addr()?;
        let public_addr = SocketAddr::new(public_ipv4.into(), local_addr.port());
        let local_v4_addr = public_addr;
        let local_v6_addr = local_addr;

        let rtc_config = RtcConfig::default()
            .set_sctp_max_message_size(5 * 1024 * 1024)
            .set_sctp_buffer_size(5 * 1024 * 1024);

        let mut rtc = rtc_config.build(Instant::now());

        if let Ok(candidate) = Candidate::host(public_addr, Protocol::Udp) {
            rtc.add_local_candidate(candidate);
        }

        let offer =
            SdpOffer::from_sdp_string(sdp_offer).map_err(|e| RelayRtcError::Rtc(str0m::error::RtcError::RemoteSdp(e.to_string())))?;

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

        let socket = RateLimitedUdpSocket::new(socket);

        let client = Box::new(Self {
            rtc,
            socket,
            local_v4_addr,
            local_v6_addr,
            buf: vec![0u8; 2000],
            cached_timeout: Instant::now(),
            pending_events,
            connected: false,
            ice_disconnected_since: None,
            upload_limit: UploadLimitState::default(),
        });

        Ok((client, sdp_answer.to_sdp_string()))
    }

    /// Poll one RTC output item and process it.
    ///
    /// Returning `MorePending` means the caller should call this method again soon,
    /// but preferably after yielding so other tasks can run.
    pub async fn poll_output(&mut self) -> Result<PollOutcome, RelayRtcError> {
        // Check ICE disconnect timeout before polling
        if let Some(since) = self.ice_disconnected_since {
            if since.elapsed() >= ICE_DISCONNECT_TIMEOUT {
                log::info!("[relay-rtc] ICE disconnected for {:?}, tearing down", ICE_DISCONNECT_TIMEOUT);
                self.rtc.disconnect();
                self.ice_disconnected_since = None;
                return Ok(PollOutcome::Idle);
            }
        }

        match self.rtc.poll_output()? {
            Output::Timeout(t) => {
                self.cached_timeout = t;
                Ok(PollOutcome::Idle)
            }
            Output::Transmit(t) => {
                let dest = to_v6_mapped(t.destination);
                let len = t.contents.len();

                if let Err(e) = self.socket.send_to(&t.contents, dest).await {
                    log::warn!("[relay-rtc] Failed to send to {}: {}", dest, e);
                } else {
                    log::trace!("[relay-rtc] Transmitted {} bytes to {}", len, dest);
                }

                Ok(PollOutcome::MorePending)
            }
            Output::Event(e) => {
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
                Ok(PollOutcome::Event(e))
            }
        }
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
                let local = if source.is_ipv4() { self.local_v4_addr } else { self.local_v6_addr };
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
        let mut pending_spins = 0usize;
        loop {
            match self.poll_output().await? {
                PollOutcome::Event(e) => return Ok(Some(e)),
                PollOutcome::MorePending => {
                    pending_spins += 1;
                    if pending_spins >= MAX_PENDING_SPINS_PER_STEP {
                        tokio::task::yield_now().await;
                        return Ok(None);
                    }
                    tokio::task::yield_now().await;
                    continue;
                }
                PollOutcome::Idle => break,
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
        self.socket.download_rate_bps()
    }

    pub fn upload_rate_bps(&mut self) -> f64 {
        self.socket.upload_rate_bps()
    }

    pub fn peer_download_rate_bps(&mut self) -> f64 {
        self.socket.upload_rate_bps()
    }

    pub fn update_upload_limit(&mut self, queued_bytes: usize, other_side_download_bps: f64) {
        let raw_other_side_download_bps = other_side_download_bps;
        if queued_bytes < QUEUE_MEASURE_START_BYTES {
            self.upload_limit.measured_upload_bps = None;
            self.socket.set_upload_speed_limit(f64::INFINITY);
            self.log_upload_limit_released(queued_bytes, raw_other_side_download_bps);
            return;
        }

        let current_upload_bps = self.download_rate_bps().max(MIN_UPLOAD_SPEED_BPS);
        let measured_upload_bps = self
            .upload_limit
            .measured_upload_bps
            .map_or(current_upload_bps, |measured| measured.max(current_upload_bps));
        self.upload_limit.measured_upload_bps = Some(measured_upload_bps);

        if queued_bytes < QUEUE_SOFT_LIMIT_BYTES {
            self.socket.set_upload_speed_limit(f64::INFINITY);
            self.log_upload_limit_released(queued_bytes, raw_other_side_download_bps);
            return;
        }

        let other_side_download_bps = other_side_download_bps.max(MIN_UPLOAD_SPEED_BPS);
        let (branch, limit_bps) = if queued_bytes < QUEUE_MATCH_TARGET_BYTES {
            let progress = (queued_bytes - QUEUE_SOFT_LIMIT_BYTES) as f64 / (QUEUE_MATCH_TARGET_BYTES - QUEUE_SOFT_LIMIT_BYTES) as f64;
            (
                "match_peer",
                interpolate(
                    measured_upload_bps.max(other_side_download_bps),
                    other_side_download_bps,
                    progress,
                ),
            )
        } else if queued_bytes < QUEUE_HARD_LIMIT_BYTES {
            let progress =
                (queued_bytes - QUEUE_MATCH_TARGET_BYTES) as f64 / (QUEUE_HARD_LIMIT_BYTES - QUEUE_MATCH_TARGET_BYTES) as f64;
            (
                "ramp_to_min",
                interpolate(other_side_download_bps, MIN_UPLOAD_SPEED_BPS, progress),
            )
        } else {
            ("min_cap", MIN_UPLOAD_SPEED_BPS)
        };

        let applied_limit_bps = limit_bps.max(MIN_UPLOAD_SPEED_BPS);
        self.socket.set_upload_speed_limit(applied_limit_bps);
        self.log_upload_limit_applied(
            branch,
            queued_bytes,
            other_side_download_bps,
            measured_upload_bps,
            current_upload_bps,
            applied_limit_bps,
        );
    }

    fn log_upload_limit_applied(
        &mut self,
        branch: &str,
        queued_bytes: usize,
        other_side_download_bps: f64,
        measured_upload_bps: f64,
        current_upload_bps: f64,
        applied_limit_bps: f64,
    ) {
        if !self.upload_limit.limit_active_logged {
            self.upload_limit.limit_active_logged = true;
            self.upload_limit.hard_cap_logged = branch == "min_cap";
            log::info!(
                concat!(
                    "[relay-rtc] upload_limit active branch={} queue={}B other_down={:.0}B/s ",
                    "measured={:.0}B/s current={:.0}B/s applied={:.0}B/s"
                ),
                branch,
                queued_bytes,
                other_side_download_bps,
                measured_upload_bps,
                current_upload_bps,
                applied_limit_bps,
            );
            return;
        }

        if branch == "min_cap" && !self.upload_limit.hard_cap_logged {
            self.upload_limit.hard_cap_logged = true;
            log::info!(
                "[relay-rtc] upload_limit hard_cap queue={}B other_down={:.0}B/s measured={:.0}B/s current={:.0}B/s applied={:.0}B/s",
                queued_bytes,
                other_side_download_bps,
                measured_upload_bps,
                current_upload_bps,
                applied_limit_bps,
            );
        }
    }

    fn log_upload_limit_released(&mut self, queued_bytes: usize, other_side_download_bps: f64) {
        if !self.upload_limit.limit_active_logged {
            return;
        }

        self.upload_limit.limit_active_logged = false;
        self.upload_limit.hard_cap_logged = false;
        log::info!(
            "[relay-rtc] upload_limit released queue={}B other_down={:.0}B/s applied=inf",
            queued_bytes,
            other_side_download_bps,
        );
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
            if octets[0..12]
                == [
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xff, 0xff,
                ]
            {
                let v4 = std::net::Ipv4Addr::new(octets[12], octets[13], octets[14], octets[15]);
                SocketAddr::new(v4.into(), v6.port())
            } else {
                addr
            }
        }
        _ => addr,
    }
}

fn to_v6_mapped(addr: SocketAddr) -> SocketAddr {
    match addr {
        SocketAddr::V4(v4) => SocketAddr::new(v4.ip().to_ipv6_mapped().into(), v4.port()),
        v6 => v6,
    }
}

fn interpolate(start: f64, end: f64, progress: f64) -> f64 {
    start + (end - start) * progress.clamp(0.0, 1.0)
}
