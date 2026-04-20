use std::collections::HashSet;
use std::net::{IpAddr, SocketAddr, ToSocketAddrs};
use std::time::{Duration, Instant};

use str0m::Candidate;
use stun_proto::agent::{StunAgent, StunAgentPollRet, Transmit};
use stun_proto::types::attribute::{AttributeType, MappedSocketAddr, XorMappedAddress};
use stun_proto::types::message::{Message, MessageWriteVec, BINDING};
use stun_proto::types::prelude::*;
use stun_proto::types::TransportType as StunTransportType;
use stun_proto::Instant as StunInstant;
use thiserror::Error;
use turn_client_proto::api::{TurnClientApi, TurnConfig};
use turn_client_proto::types::TurnCredentials;
use turn_client_proto::udp::{TurnClientUdp, TurnEvent, TurnPollRet, TurnRecvRet};

use schema::devlog::rpc_signalling::server::IceConfig;

use super::turn::{stun_now, TurnRelayInfo};

const STUN_TIMEOUT: Duration = Duration::from_millis(3000);
const TURN_TIMEOUT: Duration = Duration::from_millis(5000);
const TURN_MIN_RECV_TIMEOUT: Duration = Duration::from_millis(1);
const STUN_MAX_PACKET: usize = 512;

#[derive(Debug, Error)]
pub enum IceError {
    #[error("Candidate parsing error: {0}")]
    Parse(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("STUN error: {0}")]
    Stun(String),

    #[error("TURN error: {0}")]
    Turn(String),

    #[error("Gathering timed out")]
    Timeout,
}

fn is_usable_interface(iface: &if_addrs::Interface) -> bool {
    if iface.is_loopback() {
        return false;
    }
    if is_filtered_interface_name(&iface.name, cfg!(target_os = "windows")) {
        return false;
    }
    match iface.ip() {
        IpAddr::V4(v4) => !v4.is_link_local(),
        IpAddr::V6(v6) => (v6.segments()[0] & 0xffc0) != 0xfe80,
    }
}

fn is_filtered_interface_name(name: &str, apply_windows_rules: bool) -> bool {
    let lowered = name.trim().to_ascii_lowercase();
    if lowered.starts_with("docker")
        || lowered.starts_with("vbox")
        || lowered.starts_with("br-")
        || lowered.starts_with("veth")
        || lowered.starts_with("virbr")
    {
        return true;
    }

    apply_windows_rules && is_windows_virtual_interface_name(&lowered)
}

fn is_windows_virtual_interface_name(lowered_name: &str) -> bool {
    lowered_name.starts_with("vethernet")
        || lowered_name.contains("hyper-v")
        || lowered_name.contains("wsl")
        || lowered_name.contains("npcap")
        || lowered_name.contains("tailscale")
        || lowered_name.contains("zerotier")
        || lowered_name.contains("wireguard")
        || lowered_name.contains("loopback pseudo-interface")
        || lowered_name.contains("teredo")
        || lowered_name.contains("isatap")
        || lowered_name.contains("6to4")
        || lowered_name.contains("openvpn")
        || lowered_name.contains("hamachi")
}

fn to_v6_mapped(addr: SocketAddr) -> SocketAddr {
    match addr {
        SocketAddr::V4(v4) => SocketAddr::new(v4.ip().to_ipv6_mapped().into(), v4.port()),
        v6 => v6,
    }
}

fn from_v6_mapped(addr: SocketAddr) -> SocketAddr {
    match addr {
        SocketAddr::V6(v6) => match v6.ip().to_ipv4_mapped() {
            Some(ipv4) => SocketAddr::new(ipv4.into(), v6.port()),
            None => SocketAddr::V6(v6),
        },
        v4 => v4,
    }
}

fn parse_stun_urls(config: &IceConfig) -> Vec<String> {
    config.urls.iter().filter(|u| u.starts_with("stun:")).cloned().collect()
}

fn parse_turn_urls(config: &IceConfig) -> Vec<String> {
    config.urls.iter().filter(|u| u.starts_with("turn:")).cloned().collect()
}

pub(crate) fn stun_url_to_host_port(url: &str) -> Option<String> {
    let stripped = url.strip_prefix("stun:")?.trim();
    if stripped.is_empty() {
        return None;
    }

    if stripped.starts_with('[') {
        return if stripped.contains("]:") {
            Some(stripped.to_string())
        } else {
            Some(format!("{}:3478", stripped))
        };
    }

    if stripped.parse::<std::net::Ipv6Addr>().is_ok() {
        return Some(format!("[{}]:3478", stripped));
    }

    if stripped.rsplit_once(':').and_then(|(_, port)| port.parse::<u16>().ok()).is_some() {
        return Some(stripped.to_string());
    }

    Some(format!("{}:3478", stripped))
}

pub(crate) fn turn_url_to_host_port(url: &str) -> Option<String> {
    let stripped = url.strip_prefix("turn:")?.trim();
    if stripped.is_empty() {
        return None;
    }

    // Handle transport parameter: "turn:host?transport=udp" or "turn:host?transport=tcp"
    let stripped = stripped.split('?').next()?.trim();

    if stripped.starts_with('[') {
        return if stripped.contains("]:") {
            Some(stripped.to_string())
        } else {
            // bare IPv6 without port - shouldn't happen for TURN but handle it
            Some(format!("{}:3478", stripped))
        };
    }

    if stripped.parse::<std::net::Ipv6Addr>().is_ok() {
        return Some(format!("[{}]:3478", stripped));
    }

    // Check if host:port format
    if stripped.rsplit_once(':').and_then(|(_, port)| port.parse::<u16>().ok()).is_some() {
        return Some(stripped.to_string());
    }

    // Just hostname, default to 3478
    Some(format!("{}:3478", stripped))
}

fn turn_credentials_from_config(config: &IceConfig) -> Option<TurnCredentials> {
    let username = config.username.as_ref()?;
    let credential = config.credential.as_ref()?;
    Some(TurnCredentials::new(username, credential))
}

fn extract_mapped_addr(msg: &Message<'_>) -> Option<SocketAddr> {
    if let Ok(xma) = msg.attribute::<XorMappedAddress>() {
        return Some(xma.addr(msg.transaction_id()));
    }
    msg.raw_attribute(AttributeType::new(0x0001))
        .and_then(|raw| MappedSocketAddr::from_raw(&raw).ok())
        .map(|m| m.addr())
}

#[cfg(test)]
async fn connect_relay(
    socket: &tokio::net::UdpSocket,
    server_addr: SocketAddr,
    local_addr: SocketAddr,
    credentials: TurnCredentials,
) -> Result<TurnRelayInfo, IceError> {
    let config = TurnConfig::new(credentials);
    let client = TurnClientUdp::allocate(local_addr, server_addr, config);
    let stun_base = Instant::now();

    let mut pending = client;
    let mut buf = [0u8; STUN_MAX_PACKET];

    let start = Instant::now();
    let mut allocation_succeeded = false;
    let mut relay_addr = None;
    let mut is_closed = false;

    while !is_closed && start.elapsed() < TURN_TIMEOUT {
        let remaining = TURN_TIMEOUT.saturating_sub(start.elapsed());
        if remaining.is_zero() {
            break;
        }

        let now = stun_now(stun_base);
        let mut wait_for = remaining;
        match pending.poll(now) {
            TurnPollRet::WaitUntil(deadline) => {
                let poll_wait = deadline.checked_duration_since(now).unwrap_or(Duration::ZERO).max(TURN_MIN_RECV_TIMEOUT);
                wait_for = remaining.min(poll_wait);
            }
            TurnPollRet::Closed => {
                log::warn!("[ice] TURN client closed during allocation");
                is_closed = true;
            }
            TurnPollRet::TcpClose { .. } | TurnPollRet::AllocateTcpSocket { .. } => {
                // UDP client won't get these
            }
        }

        // Process any events
        while let Some(event) = pending.poll_event() {
            match event {
                TurnEvent::AllocationCreated(_, relayed) => {
                    relay_addr = Some(relayed);
                    allocation_succeeded = true;
                    log::info!("[ice] TURN allocation succeeded, relay addr: {}", relayed);
                }
                TurnEvent::AllocationCreateFailed(family) => {
                    log::warn!("[ice] TURN allocation failed for address family: {:?}", family);
                }
                TurnEvent::PermissionCreated(_, _) => {}
                TurnEvent::PermissionCreateFailed(_, _) => {}
                TurnEvent::ChannelCreated(_, _) => {}
                TurnEvent::ChannelCreateFailed(_, _) => {}
                TurnEvent::TcpConnected(_) => {}
                TurnEvent::TcpConnectFailed(_) => {}
            }
        }

        // Send any pending transmits
        let now = stun_now(stun_base);
        while let Some(transmit) = pending.poll_transmit(now) {
            let send_addr = to_v6_mapped(transmit.to);
            log::debug!(
                "[ice] TURN transmit to={} (mapped to={}) from={} data_len={}",
                transmit.to,
                send_addr,
                transmit.from,
                transmit.data.as_ref().len()
            );
            if let Err(e) = socket.send_to(&transmit.data, send_addr).await {
                log::warn!(
                    "[ice] TURN transmit send error: {} (to={}, mapped={})",
                    e,
                    transmit.to,
                    send_addr
                );
            }
        }

        if allocation_succeeded || is_closed {
            break;
        }

        match tokio::time::timeout(wait_for, socket.recv_from(&mut buf)).await {
            Ok(Ok((n, src))) => {
                let now = stun_now(stun_base);
                let source = from_v6_mapped(src);
                let transmit = Transmit::new(&buf[..n], StunTransportType::Udp, source, local_addr);
                match pending.recv(transmit, now) {
                    TurnRecvRet::Ignored(_) => {
                        // Not a TURN packet, might be STUN or something else - pass to str0m
                    }
                    TurnRecvRet::Handled => {
                        // TURN control message handled internally
                    }
                    TurnRecvRet::PeerData(_peer_data) => {
                        // This shouldn't happen during allocation - peer data comes after allocation
                        log::warn!("[ice] Unexpected peer data during TURN allocation from {}", source);
                    }
                    TurnRecvRet::PeerIcmp { .. } => {
                        // ICMP error - ignore
                    }
                }
            }
            Ok(Err(e)) => {
                log::warn!("[ice] Socket recv error during TURN allocation: {}", e);
            }
            Err(_) => {
                // Timeout on recv - loop so the TURN state machine can retransmit.
            }
        }
    }

    if !allocation_succeeded {
        return Err(IceError::Timeout);
    }

    let relay_addr = relay_addr.ok_or(IceError::Timeout)?;

    Ok(TurnRelayInfo::new(pending, server_addr, relay_addr, stun_base))
}

#[derive(Default)]
struct PhaseDrive {
    outbound: Vec<OutboundPacket>,
    wait: Option<Duration>,
}

struct OutboundPacket {
    data: Vec<u8>,
    to: SocketAddr,
}

struct StunTransaction {
    agent: StunAgent,
    server_addr: SocketAddr,
}

struct StunPhase {
    local_addr: SocketAddr,
    start: Instant,
    stun_base: Instant,
    pending: Vec<StunTransaction>,
    queued: Vec<OutboundPacket>,
    successes: usize,
    completed: bool,
    failure: Option<String>,
}

impl StunPhase {
    fn start(local_addr: SocketAddr, urls: &[String]) -> Result<Self, IceError> {
        if urls.is_empty() {
            return Err(IceError::Stun("no STUN server URLs configured".to_string()));
        }

        let stun_base = Instant::now();
        let now = StunInstant::from_std(stun_base);
        let mut pending = Vec::new();
        let mut queued = Vec::new();
        let mut seen = HashSet::new();

        for url_str in urls {
            if let Some(host_port) = stun_url_to_host_port(url_str) {
                if let Ok(addrs) = host_port.to_socket_addrs() {
                    for stun_addr in addrs {
                        let send_addr = to_v6_mapped(stun_addr);
                        if !seen.insert(send_addr) {
                            continue;
                        }

                        let mut agent = StunAgent::builder(StunTransportType::Udp, local_addr).remote_addr(send_addr).build();
                        let msg = Message::builder_request(BINDING, MessageWriteVec::new()).finish();
                        match agent.send_request(msg, send_addr, now) {
                            Ok(transmit) => {
                                queued.push(transmit_to_packet(transmit));
                                pending.push(StunTransaction {
                                    agent,
                                    server_addr: stun_addr,
                                });
                            }
                            Err(e) => {
                                log::warn!("[ice] send_request for {stun_addr}: {e:?}");
                            }
                        }
                    }
                }
            }
        }

        if pending.is_empty() {
            return Err(IceError::Stun("no STUN requests could be started".to_string()));
        }

        Ok(Self {
            local_addr,
            start: Instant::now(),
            stun_base,
            pending,
            queued,
            successes: 0,
            completed: false,
            failure: None,
        })
    }

    fn is_complete(&self) -> bool {
        self.completed
    }

    fn failure(&self) -> Option<&str> {
        self.failure.as_deref()
    }

    fn drive(&mut self) -> PhaseDrive {
        if self.completed || self.failure.is_some() {
            return PhaseDrive::default();
        }

        let mut drive = PhaseDrive {
            outbound: self.queued.drain(..).collect(),
            wait: None,
        };

        let elapsed = self.start.elapsed();
        if elapsed >= STUN_TIMEOUT {
            self.fail("STUN gathering timed out".to_string());
            return drive;
        }

        let remaining = STUN_TIMEOUT.saturating_sub(elapsed);
        let now = StunInstant::from_std(self.stun_base);
        let mut next_wait = remaining;
        let mut saw_wait = false;

        self.pending.retain_mut(|transaction| match transaction.agent.poll(now) {
            StunAgentPollRet::TransactionTimedOut(_) | StunAgentPollRet::TransactionCancelled(_) => {
                log::warn!("[ice] STUN timed out for {}", transaction.server_addr);
                false
            }
            StunAgentPollRet::WaitUntil(deadline) => {
                let poll_wait = deadline.checked_duration_since(now).unwrap_or(Duration::ZERO).max(TURN_MIN_RECV_TIMEOUT);
                next_wait = next_wait.min(poll_wait);
                saw_wait = true;
                true
            }
        });

        for transaction in &mut self.pending {
            while let Some(transmit) = transaction.agent.poll_transmit(now) {
                drive.outbound.push(transmit_to_packet(transmit));
            }
        }

        if self.pending.is_empty() {
            self.complete_or_fail();
            return drive;
        }

        drive.wait = Some(if saw_wait {
            next_wait
        } else {
            remaining.max(TURN_MIN_RECV_TIMEOUT)
        });
        drive
    }

    fn handle_packet(&mut self, packet: &[u8], src: SocketAddr) -> Vec<Candidate> {
        if self.completed || self.failure.is_some() {
            return Vec::new();
        }

        let Ok(msg) = Message::from_bytes(packet) else {
            return Vec::new();
        };
        if !msg.is_response() {
            return Vec::new();
        }

        let mut matched = None;
        for (idx, transaction) in self.pending.iter_mut().enumerate() {
            if transaction.agent.handle_stun_message(&msg, src) {
                matched = Some(idx);
                break;
            }
        }

        let Some(idx) = matched else {
            return Vec::new();
        };

        self.pending.swap_remove(idx);

        let mut candidates = Vec::new();
        if let Some(mapped) = extract_mapped_addr(&msg) {
            let mut base = self.local_addr;
            if mapped.is_ipv4() && base.is_ipv6() {
                base = SocketAddr::new(std::net::Ipv4Addr::UNSPECIFIED.into(), base.port());
            } else if mapped.is_ipv6() && base.is_ipv4() {
                base = SocketAddr::new(std::net::Ipv6Addr::UNSPECIFIED.into(), base.port());
            }
            match Candidate::server_reflexive(mapped, base, "udp") {
                Ok(candidate) => {
                    log::info!("[ice] STUN binding succeeded with {}, candidate: {}", src, candidate);
                    self.successes += 1;
                    candidates.push(candidate);
                }
                Err(e) => {
                    log::warn!("[ice] Srflx for {mapped}: {e:?}");
                }
            }
        }

        if self.pending.is_empty() {
            self.complete_or_fail();
        }

        candidates
    }

    fn complete_or_fail(&mut self) {
        if self.successes > 0 {
            self.completed = true;
            log::info!(
                "[ice] STUN phase completed in {:?} with {} successful binding(s)",
                self.start.elapsed(),
                self.successes
            );
        } else {
            self.fail("no STUN bindings succeeded".to_string());
        }
    }

    fn fail(&mut self, reason: String) {
        if self.failure.is_none() {
            log::warn!("[ice] STUN phase failed in {:?}: {}", self.start.elapsed(), reason);
            self.failure = Some(reason);
        }
        self.pending.clear();
        self.queued.clear();
    }
}

enum TurnAttemptState {
    Pending,
    Succeeded,
    Failed(String),
}

struct TurnAttempt {
    server_addr: SocketAddr,
    local_addr: SocketAddr,
    client: TurnClientUdp,
    stun_base: Instant,
    relay_addr: Option<SocketAddr>,
    state: TurnAttemptState,
}

impl TurnAttempt {
    fn new(local_addr: SocketAddr, server_addr: SocketAddr, credentials: TurnCredentials) -> Self {
        let config = TurnConfig::new(credentials);
        let client = TurnClientUdp::allocate(local_addr, server_addr, config);
        Self {
            server_addr,
            local_addr,
            client,
            stun_base: Instant::now(),
            relay_addr: None,
            state: TurnAttemptState::Pending,
        }
    }

    fn drive(&mut self, phase_remaining: Duration) -> PhaseDrive {
        if !self.is_pending() {
            return PhaseDrive::default();
        }

        let now = stun_now(self.stun_base);
        let mut next_wait = phase_remaining;
        let mut saw_wait = false;
        match self.client.poll(now) {
            TurnPollRet::WaitUntil(deadline) => {
                let poll_wait = deadline.checked_duration_since(now).unwrap_or(Duration::ZERO).max(TURN_MIN_RECV_TIMEOUT);
                next_wait = phase_remaining.min(poll_wait);
                saw_wait = true;
            }
            TurnPollRet::Closed => {
                self.mark_failed("TURN client closed during allocation".to_string());
            }
            TurnPollRet::TcpClose { .. } | TurnPollRet::AllocateTcpSocket { .. } => {}
        }

        self.process_events();

        let mut drive = PhaseDrive::default();
        let now = stun_now(self.stun_base);
        while let Some(transmit) = self.client.poll_transmit(now) {
            drive.outbound.push(turn_transmit_to_packet(transmit));
        }

        if self.is_pending() {
            drive.wait = Some(if saw_wait {
                next_wait
            } else {
                phase_remaining.max(TURN_MIN_RECV_TIMEOUT)
            });
        }
        drive
    }

    fn handle_packet(&mut self, packet: &[u8], src: SocketAddr) -> bool {
        if !self.is_pending() {
            return false;
        }

        let now = stun_now(self.stun_base);
        let transmit = Transmit::new(packet, StunTransportType::Udp, src, self.local_addr);
        let handled = match self.client.recv(transmit, now) {
            TurnRecvRet::Ignored(_) => false,
            TurnRecvRet::Handled => true,
            TurnRecvRet::PeerData(_peer_data) => {
                log::warn!("[ice] Unexpected peer data during TURN allocation from {}", src);
                true
            }
            TurnRecvRet::PeerIcmp { .. } => true,
        };

        if handled {
            self.process_events();
        }

        handled
    }

    fn is_pending(&self) -> bool {
        matches!(self.state, TurnAttemptState::Pending)
    }

    fn is_succeeded(&self) -> bool {
        matches!(self.state, TurnAttemptState::Succeeded)
    }

    fn failure_reason(&self) -> Option<&str> {
        match &self.state {
            TurnAttemptState::Failed(reason) => Some(reason.as_str()),
            TurnAttemptState::Pending | TurnAttemptState::Succeeded => None,
        }
    }

    fn into_relay_info(self) -> Option<TurnRelayInfo> {
        if !matches!(self.state, TurnAttemptState::Succeeded) {
            return None;
        }
        Some(TurnRelayInfo::new(
            self.client,
            self.server_addr,
            self.relay_addr?,
            self.stun_base,
        ))
    }

    fn process_events(&mut self) {
        while let Some(event) = self.client.poll_event() {
            match event {
                TurnEvent::AllocationCreated(_, relayed) => {
                    self.relay_addr = Some(relayed);
                    self.state = TurnAttemptState::Succeeded;
                    log::info!("[ice] TURN allocation succeeded, relay addr: {}", relayed);
                }
                TurnEvent::AllocationCreateFailed(family) => {
                    log::warn!("[ice] TURN allocation failed for address family: {:?}", family);
                }
                TurnEvent::PermissionCreated(_, _) => {}
                TurnEvent::PermissionCreateFailed(_, _) => {}
                TurnEvent::ChannelCreated(_, _) => {}
                TurnEvent::ChannelCreateFailed(_, _) => {}
                TurnEvent::TcpConnected(_) => {}
                TurnEvent::TcpConnectFailed(_) => {}
            }
        }
    }

    fn mark_failed(&mut self, reason: String) {
        if self.is_pending() {
            self.state = TurnAttemptState::Failed(reason);
        }
    }
}

struct TurnPhase {
    start: Instant,
    attempts: Vec<TurnAttempt>,
    success: Option<TurnRelayInfo>,
    failure: Option<String>,
}

impl TurnPhase {
    fn start(local_addr: SocketAddr, urls: &[String], credentials: TurnCredentials) -> Result<Self, IceError> {
        if urls.is_empty() {
            return Err(IceError::Turn("no TURN server URLs configured".to_string()));
        }

        let mut attempts = Vec::new();
        let mut seen = HashSet::new();

        for turn_url in urls {
            if let Some(host_port) = turn_url_to_host_port(turn_url) {
                if let Ok(addrs) = host_port.to_socket_addrs() {
                    for turn_server_addr in addrs {
                        if !seen.insert(turn_server_addr) {
                            continue;
                        }
                        log::info!("[ice] Attempting TURN allocation to {}", turn_server_addr);
                        attempts.push(TurnAttempt::new(local_addr, turn_server_addr, credentials.clone()));
                    }
                }
            }
        }

        if attempts.is_empty() {
            return Err(IceError::Turn("no TURN allocation attempts could be started".to_string()));
        }

        Ok(Self {
            start: Instant::now(),
            attempts,
            success: None,
            failure: None,
        })
    }

    fn is_complete(&self) -> bool {
        self.success.is_some()
    }

    fn failure(&self) -> Option<&str> {
        self.failure.as_deref()
    }

    fn take_success(&mut self) -> Option<TurnRelayInfo> {
        self.success.take()
    }

    fn drive(&mut self) -> PhaseDrive {
        if self.success.is_some() || self.failure.is_some() {
            return PhaseDrive::default();
        }

        let elapsed = self.start.elapsed();
        if elapsed >= TURN_TIMEOUT {
            self.fail(format!("TURN gathering timed out after {:?}", TURN_TIMEOUT));
            return PhaseDrive::default();
        }

        let remaining = TURN_TIMEOUT.saturating_sub(elapsed);
        let mut drive = PhaseDrive::default();
        let mut next_wait = remaining;
        let mut saw_wait = false;
        let mut idx = 0;

        while idx < self.attempts.len() {
            let attempt_drive = self.attempts[idx].drive(remaining);
            drive.outbound.extend(attempt_drive.outbound);

            if let Some(wait) = attempt_drive.wait {
                next_wait = next_wait.min(wait);
                saw_wait = true;
            }

            if self.try_capture_success(idx) {
                log::info!("[ice] TURN phase completed in {:?}", self.start.elapsed());
                return drive;
            }

            if let Some(reason) = self.attempts[idx].failure_reason().map(ToOwned::to_owned) {
                let server_addr = self.attempts[idx].server_addr;
                self.attempts.swap_remove(idx);
                log::warn!("[ice] TURN allocation failed for {}: {}", server_addr, reason);
                continue;
            }

            idx += 1;
        }

        if self.attempts.is_empty() {
            self.fail("all TURN allocation attempts failed".to_string());
            return drive;
        }

        drive.wait = Some(if saw_wait {
            next_wait
        } else {
            remaining.max(TURN_MIN_RECV_TIMEOUT)
        });
        drive
    }

    fn handle_packet(&mut self, packet: &[u8], src: SocketAddr) -> bool {
        if self.success.is_some() || self.failure.is_some() {
            return false;
        }

        let mut handled = false;
        let mut idx = 0;
        while idx < self.attempts.len() {
            handled |= self.attempts[idx].handle_packet(packet, src);

            if self.try_capture_success(idx) {
                log::info!("[ice] TURN phase completed in {:?}", self.start.elapsed());
                return true;
            }

            if let Some(reason) = self.attempts[idx].failure_reason().map(ToOwned::to_owned) {
                let server_addr = self.attempts[idx].server_addr;
                self.attempts.swap_remove(idx);
                log::warn!("[ice] TURN allocation failed for {}: {}", server_addr, reason);
                continue;
            }

            idx += 1;
        }

        if self.attempts.is_empty() {
            self.fail("all TURN allocation attempts failed".to_string());
        }

        handled
    }

    fn try_capture_success(&mut self, idx: usize) -> bool {
        if !self.attempts[idx].is_succeeded() {
            return false;
        }

        let server_addr = self.attempts[idx].server_addr;
        let relay_addr = self.attempts[idx].relay_addr.expect("successful TURN attempt missing relay addr");
        let attempt = self.attempts.swap_remove(idx);
        self.success = attempt.into_relay_info();
        self.attempts.clear();
        log::info!(
            "[ice] TURN allocation succeeded with server {}, relay: {}",
            server_addr,
            relay_addr
        );
        true
    }

    fn fail(&mut self, reason: String) {
        if self.failure.is_none() {
            log::warn!("[ice] TURN phase failed in {:?}: {}", self.start.elapsed(), reason);
            self.failure = Some(reason);
        }
        self.attempts.clear();
    }
}

fn transmit_to_packet<T: AsRef<[u8]>>(transmit: Transmit<T>) -> OutboundPacket {
    OutboundPacket {
        data: transmit.data.as_ref().to_vec(),
        to: transmit.to,
    }
}

fn turn_transmit_to_packet<T: AsRef<[u8]>>(transmit: Transmit<T>) -> OutboundPacket {
    let send_addr = to_v6_mapped(transmit.to);
    log::debug!(
        "[ice] TURN transmit to={} (mapped to={}) from={} data_len={}",
        transmit.to,
        send_addr,
        transmit.from,
        transmit.data.as_ref().len()
    );
    OutboundPacket {
        data: transmit.data.as_ref().to_vec(),
        to: send_addr,
    }
}

fn min_wait(a: Option<Duration>, b: Option<Duration>) -> Option<Duration> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

async fn send_outbound_packets(socket: &tokio::net::UdpSocket, packets: Vec<OutboundPacket>) -> Result<(), IceError> {
    for packet in packets {
        if let Err(error) = socket.send_to(&packet.data, packet.to).await {
            log::warn!("[ice] Send to {} failed during candidate gathering: {}", packet.to, error);
        }
    }

    Ok(())
}

pub struct IceAgent;

impl IceAgent {
    pub async fn resolve_remote_candidates(sdp: &str) -> String {
        use std::collections::HashMap;

        let lines: Vec<&str> = sdp.lines().collect();

        let needs_resolution: Vec<(usize, String, String)> = lines
            .iter()
            .enumerate()
            .filter_map(|(idx, line)| {
                if line.contains("candidate:") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() > 5 {
                        let hostname = parts[4];
                        if hostname.parse::<std::net::IpAddr>().is_err() {
                            return Some((idx, hostname.to_string(), parts[5].to_string()));
                        }
                    }
                }
                None
            })
            .collect();

        let handles: Vec<_> = needs_resolution
            .iter()
            .map(|(idx, hostname, port)| {
                let hostname = hostname.clone();
                let port = port.clone();
                let idx = *idx;
                tokio::task::spawn_blocking(move || {
                    let lookup = format!("{}:{}", hostname, port);
                    (lookup.to_socket_addrs(), hostname, idx)
                })
            })
            .collect();

        let mut resolved: HashMap<(usize, String), String> = HashMap::new();
        for h in handles {
            match h.await {
                Ok((result, hostname, idx)) => {
                    match result {
                        Ok(mut addrs) => {
                            if let Some(resolved_addr) = addrs.next() {
                                resolved.insert((idx, hostname), resolved_addr.ip().to_string());
                            }
                        }
                        Err(e) => {
                            log::warn!("[ice] Failed to resolve remote candidate {}: {}", hostname, e);
                        }
                    }
                }
                Err(e) => {
                    log::error!("[ice] spawn_blocking join error: {}", e);
                }
            }
        }

        let mut resolved_lines = Vec::new();
        for (idx, line) in lines.iter().enumerate() {
            if line.contains("candidate:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() > 5 {
                    let hostname = parts[4];
                    if hostname.parse::<std::net::IpAddr>().is_err() {
                        if let Some(ip) = resolved.get(&(idx, hostname.to_string())) {
                            let mut new_parts = parts;
                            new_parts[4] = ip;
                            resolved_lines.push(new_parts.join(" "));
                            continue;
                        }
                    }
                }
            }
            resolved_lines.push(line.to_string());
        }
        resolved_lines.join("\r\n")
    }

    pub async fn gather_candidates(
        socket: &tokio::net::UdpSocket,
        config: &IceConfig,
    ) -> Result<(Vec<Candidate>, Option<TurnRelayInfo>), IceError> {
        log::info!("[ice] Gathering candidates using config {config:?}");
        let gather_start = Instant::now();

        let mut candidates: HashSet<Candidate> = HashSet::new();
        let local_port = socket.local_addr().map(|a| a.port()).unwrap_or(0);
        let host_start = Instant::now();

        if let Ok(ifaces) = if_addrs::get_if_addrs() {
            for iface in ifaces {
                if !is_usable_interface(&iface) {
                    continue;
                }
                let addr = SocketAddr::new(iface.ip(), local_port);
                match Candidate::host(addr, "udp") {
                    Ok(c) => {
                        log::debug!("[ice] Host candidate: {}", c);
                        candidates.insert(c);
                    }
                    Err(e) => {
                        log::warn!("[ice] Failed to create host candidate for {}: {:?}", addr, e);
                    }
                }
            }
        }
        log::info!("[ice] Host candidate enumeration completed in {:?}", host_start.elapsed());

        let local_addr = socket.local_addr().unwrap_or_else(|_| SocketAddr::new(std::net::Ipv4Addr::UNSPECIFIED.into(), 0));
        let mut stun_phase = StunPhase::start(local_addr, &parse_stun_urls(config))?;
        let credentials =
            turn_credentials_from_config(config).ok_or_else(|| IceError::Turn("missing TURN credentials".to_string()))?;
        let mut turn_phase = TurnPhase::start(local_addr, &parse_turn_urls(config), credentials)?;
        let mut buf = [0u8; STUN_MAX_PACKET];

        loop {
            let stun_drive = stun_phase.drive();
            let turn_drive = turn_phase.drive();
            let mut outbound = stun_drive.outbound;
            outbound.extend(turn_drive.outbound);
            send_outbound_packets(socket, outbound).await?;

            if let Some(reason) = stun_phase.failure() {
                return Err(IceError::Stun(reason.to_string()));
            }
            if let Some(reason) = turn_phase.failure() {
                return Err(IceError::Turn(reason.to_string()));
            }

            if stun_phase.is_complete() && turn_phase.is_complete() {
                let turn_info = turn_phase.take_success().expect("TURN phase completed without relay info");
                let relay_addr = turn_info.relay_addr;
                match Candidate::relayed(relay_addr, relay_addr, "udp") {
                    Ok(candidate) => {
                        log::debug!("[ice] Relayed candidate: {}", candidate);
                        candidates.insert(candidate);
                    }
                    Err(error) => {
                        return Err(IceError::Parse(format!(
                            "failed to create relayed candidate for {relay_addr}: {error:?}"
                        )));
                    }
                }

                let result: Vec<Candidate> = candidates.into_iter().collect();
                log::info!("[ice] Gathered {:?} candidates in {:?}", result, gather_start.elapsed());
                return Ok((result, Some(turn_info)));
            }

            let wait_for = min_wait(stun_drive.wait, turn_drive.wait)
                .unwrap_or(TURN_MIN_RECV_TIMEOUT)
                .max(TURN_MIN_RECV_TIMEOUT);

            match tokio::time::timeout(wait_for, socket.recv_from(&mut buf)).await {
                Ok(Ok((n, src))) => {
                    for candidate in stun_phase.handle_packet(&buf[..n], src) {
                        candidates.insert(candidate);
                    }
                    turn_phase.handle_packet(&buf[..n], from_v6_mapped(src));
                }
                Ok(Err(error)) => {
                    log::warn!("[ice] Socket recv error during candidate gathering: {}", error);
                }
                Err(_) => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        connect_relay, from_v6_mapped, is_filtered_interface_name, is_windows_virtual_interface_name, IceAgent, IceError,
        STUN_MAX_PACKET, STUN_TIMEOUT, TURN_TIMEOUT,
    };
    use socket2::{Domain, Socket, Type};
    use std::collections::HashMap;
    use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV6};
    use std::time::Duration;
    use tokio::time::{sleep, timeout, Instant};
    use turn_client_proto::types::TurnCredentials;
    use turn_server::config::{Auth, Config, Interface, Log, Server};
    use turn_server::start_server;

    use schema::devlog::rpc_signalling::server::IceConfig;
    use stun_proto::types::attribute::XorMappedAddress;
    use stun_proto::types::message::{Message, MessageWrite, MessageWriteExt, MessageWriteVec, BINDING};

    async fn make_dual_stack_socket() -> tokio::net::UdpSocket {
        let client_socket = Socket::new(Domain::IPV6, Type::DGRAM, Some(socket2::Protocol::UDP)).unwrap();
        client_socket.set_only_v6(false).unwrap();
        client_socket.set_nonblocking(true).unwrap();
        client_socket.bind(&"[::]:0".parse::<SocketAddr>().unwrap().into()).unwrap();
        let client_socket: std::net::UdpSocket = client_socket.into();
        tokio::net::UdpSocket::from_std(client_socket).unwrap()
    }

    async fn spawn_blackhole_server() -> SocketAddr {
        let socket = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let addr = socket.local_addr().unwrap();
        tokio::spawn(async move {
            let mut buf = [0u8; 1500];
            loop {
                let _ = socket.recv_from(&mut buf).await;
            }
        });
        addr
    }

    async fn spawn_stun_server() -> SocketAddr {
        let socket = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let addr = socket.local_addr().unwrap();
        tokio::spawn(async move {
            let mut buf = [0u8; 1500];
            loop {
                let (size, from) = match socket.recv_from(&mut buf).await {
                    Ok(result) => result,
                    Err(_) => continue,
                };
                let Ok(message) = Message::from_bytes(&buf[..size]) else {
                    continue;
                };
                if message.is_response() || !message.has_method(BINDING) {
                    continue;
                }

                let mut response = Message::builder_success(&message, MessageWriteVec::new());
                let xor_addr = XorMappedAddress::new(from, message.transaction_id());
                response.add_attribute(&xor_addr).unwrap();
                let response = response.finish();
                let _ = socket.send_to(&response, from).await;
            }
        });
        addr
    }

    async fn spawn_turn_server_for_tests() -> SocketAddr {
        let listener = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);

        tokio::spawn(async move {
            start_server(Config {
                log: Log::default(),
                server: Server {
                    realm: "localhost".to_string(),
                    interfaces: vec![Interface::Udp {
                        external: addr,
                        listen: addr,
                        idle_timeout: 30,
                        mtu: 1500,
                    }],
                    ..Default::default()
                },
                auth: Auth {
                    enable_hooks_auth: false,
                    static_auth_secret: Some("relay-secret".to_string()),
                    static_credentials: {
                        let mut creds = HashMap::with_capacity(1);
                        creds.insert("relay".to_string(), "relay-secret".to_string());
                        creds
                    },
                },
                api: None,
                ..Default::default()
            })
            .await
            .unwrap();
        });

        sleep(Duration::from_millis(300)).await;
        addr
    }

    fn test_ice_config(stun_addrs: &[SocketAddr], turn_addrs: &[SocketAddr]) -> IceConfig {
        let mut urls = Vec::new();
        urls.extend(stun_addrs.iter().map(|addr| format!("stun:{addr}")));
        urls.extend(turn_addrs.iter().map(|addr| format!("turn:{addr}")));
        IceConfig {
            urls,
            username: Some("relay".to_string()),
            credential: Some("relay-secret".to_string()),
        }
    }

    #[test]
    fn filters_common_windows_virtual_adapter_names() {
        for name in [
            "vEthernet (Default Switch)",
            "Hyper-V Virtual Ethernet Adapter",
            "WSL (Hyper-V firewall)",
            "Npcap Loopback Adapter",
            "Tailscale Tunnel",
            "ZeroTier One [ab12cd34ef]",
            "WireGuard Tunnel",
            "Microsoft ISATAP Adapter",
            "Teredo Tunneling Pseudo-Interface",
            "OpenVPN TAP-Windows6",
            "Hamachi",
        ] {
            assert!(is_windows_virtual_interface_name(&name.to_ascii_lowercase()), "{name}");
            assert!(is_filtered_interface_name(name, true), "{name}");
        }
    }

    #[test]
    fn keeps_physical_lan_adapter_names() {
        for name in [
            "Ethernet",
            "Ethernet 2",
            "Wi-Fi",
            "Intel(R) Ethernet Controller I225-V",
            "Realtek Gaming 2.5GbE Family Controller",
            "en0",
            "eth0",
            "wlan0",
        ] {
            assert!(!is_filtered_interface_name(name, true), "{name}");
            assert!(!is_filtered_interface_name(name, false), "{name}");
        }
    }

    #[test]
    fn windows_rules_do_not_change_non_windows_generic_filtering() {
        for name in [
            "docker0",
            "vboxnet0",
            "br-4c0f6d9a33f3",
            "veth7f2a",
            "virbr0",
        ] {
            assert!(is_filtered_interface_name(name, false), "{name}");
            assert!(is_filtered_interface_name(name, true), "{name}");
        }
    }

    #[test]
    fn collapses_ipv4_mapped_ipv6_addresses() {
        let mapped = SocketAddr::V6(SocketAddrV6::new(Ipv4Addr::LOCALHOST.to_ipv6_mapped(), 19101, 0, 0));
        let normalized = from_v6_mapped(mapped);

        assert_eq!(normalized, SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 19101));
        assert_eq!(
            from_v6_mapped(SocketAddr::new(Ipv6Addr::LOCALHOST.into(), 19101)),
            SocketAddr::new(Ipv6Addr::LOCALHOST.into(), 19101)
        );
    }

    #[tokio::test]
    async fn turn_allocation_sends_initial_request_promptly() {
        let server_socket = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let server_addr = server_socket.local_addr().unwrap();

        let client_socket = Socket::new(Domain::IPV6, Type::DGRAM, Some(socket2::Protocol::UDP)).unwrap();
        client_socket.set_only_v6(false).unwrap();
        client_socket.set_nonblocking(true).unwrap();
        client_socket.bind(&"[::]:0".parse::<SocketAddr>().unwrap().into()).unwrap();
        let client_socket: std::net::UdpSocket = client_socket.into();
        let client_socket = tokio::net::UdpSocket::from_std(client_socket).unwrap();
        let local_addr = client_socket.local_addr().unwrap();

        let turn_future = connect_relay(
            &client_socket,
            server_addr,
            local_addr,
            TurnCredentials::new("relay", "relay-secret"),
        );
        tokio::pin!(turn_future);

        let mut buf = [0u8; STUN_MAX_PACKET];
        let received = timeout(Duration::from_millis(500), async {
            tokio::select! {
                result = server_socket.recv_from(&mut buf) => result.map(|(size, _)| size),
                _ = &mut turn_future => Err(std::io::Error::other("connect_relay returned before sending")),
            }
        })
        .await
        .expect("timed out waiting for initial TURN allocation transmit")
        .expect("failed to receive initial TURN allocation transmit");

        assert!(received > 0, "expected TURN allocation request bytes");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn gather_candidates_races_turn_attempts_and_keeps_first_success() {
        let socket = make_dual_stack_socket().await;
        let stun_addr = spawn_stun_server().await;
        let blackhole_addr = spawn_blackhole_server().await;
        let turn_addr = spawn_turn_server_for_tests().await;
        let config = test_ice_config(
            &[stun_addr],
            &[
                blackhole_addr,
                turn_addr,
            ],
        );

        let started = Instant::now();
        let (candidates, turn_info) = IceAgent::gather_candidates(&socket, &config).await.unwrap();
        let elapsed = started.elapsed();

        assert!(
            elapsed < Duration::from_secs(4),
            "parallel TURN gather should not wait for the blackhole address: {elapsed:?}"
        );

        let turn_info = turn_info.expect("expected TURN relay info");
        assert_eq!(turn_info.server_addr, turn_addr);

        let candidate_strings: Vec<String> = candidates.iter().map(ToString::to_string).collect();
        assert!(
            candidate_strings.iter().any(|candidate| candidate.contains(" typ srflx")),
            "expected at least one srflx candidate in {candidate_strings:?}"
        );
        assert!(
            candidate_strings.iter().any(|candidate| candidate.contains(" typ relay")),
            "expected a relay candidate in {candidate_strings:?}"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn gather_candidates_fails_when_stun_phase_never_succeeds() {
        let socket = make_dual_stack_socket().await;
        let blackhole_stun_addr = spawn_blackhole_server().await;
        let turn_addr = spawn_turn_server_for_tests().await;
        let config = test_ice_config(&[blackhole_stun_addr], &[turn_addr]);

        let started = Instant::now();
        let error = IceAgent::gather_candidates(&socket, &config).await.unwrap_err();
        let elapsed = started.elapsed();

        assert!(matches!(error, IceError::Stun(_)));
        assert!(
            elapsed >= STUN_TIMEOUT,
            "expected STUN phase to wait for its timeout, got {elapsed:?}"
        );
        assert!(
            elapsed < TURN_TIMEOUT,
            "expected STUN failure to end gather before TURN timeout, got {elapsed:?}"
        );
    }
}
