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
const STUN_MAX_PACKET: usize = 512;

#[derive(Debug, Error)]
pub enum IceError {
    #[error("Candidate parsing error: {0}")]
    Parse(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("STUN error: {0}")]
    Stun(String),

    #[error("Gathering timed out")]
    Timeout
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
        IpAddr::V6(v6) => (v6.segments()[0] & 0xffc0) != 0xfe80
    }
}

fn is_filtered_interface_name(name: &str, apply_windows_rules: bool) -> bool {
    let lowered = name.trim().to_ascii_lowercase();
    if lowered.starts_with("docker") ||
        lowered.starts_with("vbox") ||
        lowered.starts_with("br-") ||
        lowered.starts_with("veth") ||
        lowered.starts_with("virbr")
    {
        return true;
    }

    apply_windows_rules && is_windows_virtual_interface_name(&lowered)
}

fn is_windows_virtual_interface_name(lowered_name: &str) -> bool {
    lowered_name.starts_with("vethernet") ||
        lowered_name.contains("hyper-v") ||
        lowered_name.contains("wsl") ||
        lowered_name.contains("npcap") ||
        lowered_name.contains("tailscale") ||
        lowered_name.contains("zerotier") ||
        lowered_name.contains("wireguard") ||
        lowered_name.contains("loopback pseudo-interface") ||
        lowered_name.contains("teredo") ||
        lowered_name.contains("isatap") ||
        lowered_name.contains("6to4") ||
        lowered_name.contains("openvpn") ||
        lowered_name.contains("hamachi")
}

fn to_v6_mapped(addr: SocketAddr) -> SocketAddr {
    match addr {
        SocketAddr::V4(v4) => SocketAddr::new(v4.ip().to_ipv6_mapped().into(), v4.port()),
        v6 => v6
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

        // Try to recv from socket with timeout
        match tokio::time::timeout(remaining, socket.recv_from(&mut buf)).await {
            Ok(Ok((n, src))) => {
                let now = stun_now(stun_base);
                let transmit = Transmit::new(
                    &buf[..n],
                    StunTransportType::Udp,
                    src,
                    local_addr,
                );
                match pending.recv(transmit, now) {
                    TurnRecvRet::Ignored(_) => {
                        // Not a TURN packet, might be STUN or something else - pass to str0m
                    }
                    TurnRecvRet::Handled => {
                        // TURN control message handled internally
                    }
                    TurnRecvRet::PeerData(_peer_data) => {
                        // This shouldn't happen during allocation - peer data comes after allocation
                        log::warn!("[ice] Unexpected peer data during TURN allocation from {}", src);
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
                // Timeout on recv - continue to poll
            }
        }

        let now = stun_now(stun_base);
        match pending.poll(now) {
            TurnPollRet::WaitUntil(_) => {}
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
            if let Err(e) = socket.send_to(&transmit.data, transmit.to).await {
                log::warn!("[ice] TURN transmit send error: {}", e);
            }
        }
    }

    if !allocation_succeeded {
        return Err(IceError::Timeout);
    }

    let relay_addr = relay_addr.ok_or(IceError::Timeout)?;

    Ok(TurnRelayInfo::new(pending, server_addr, relay_addr, stun_base))
}

pub struct IceAgent;

impl IceAgent {
    pub fn resolve_remote_candidates(sdp: &str) -> String {
        let mut resolved_lines = Vec::new();
        for line in sdp.lines() {
            if line.contains("candidate:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() > 5 {
                    let hostname = parts[4];
                    if hostname.parse::<std::net::IpAddr>().is_err() {
                        let port = parts[5];
                        let lookup = format!("{}:{}", hostname, port);
                        match lookup.to_socket_addrs() {
                            Ok(mut addrs) => {
                                if let Some(resolved) = addrs.next() {
                                    let mut new_parts = parts;
                                    let ip_str = resolved.ip().to_string();
                                    new_parts[4] = &ip_str;
                                    resolved_lines.push(new_parts.join(" "));
                                    continue;
                                }
                            }
                            Err(e) => {
                                log::warn!("[ice] Failed to resolve remote candidate {}: {}", hostname, e);
                            }
                        }
                    }
                }
            }
            resolved_lines.push(line.to_string());
        }
        resolved_lines.join("\r\n")
    }

    pub async fn gather_candidates(socket: &tokio::net::UdpSocket, config: &IceConfig) -> Result<(Vec<Candidate>, Option<TurnRelayInfo>), IceError> {
        log::info!("[ice] Gathering candidates using config {config:?}");

        let mut candidates: HashSet<Candidate> = HashSet::new();
        let local_port = socket.local_addr().map(|a| a.port()).unwrap_or(0);

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

        let stun_urls = parse_stun_urls(config);
        if !stun_urls.is_empty() {
            let local_addr = socket.local_addr().unwrap_or_else(|_| SocketAddr::new(std::net::Ipv4Addr::UNSPECIFIED.into(), 0));

            let stun_base = Instant::now();
            let stun_now = || StunInstant::from_std(stun_base);

            let mut pending: Vec<(StunAgent, SocketAddr)> = Vec::new();

            for url_str in &stun_urls {
                if let Some(host_port) = stun_url_to_host_port(url_str) {
                    if let Ok(addrs) = host_port.to_socket_addrs() {
                        for stun_addr in addrs {
                            let send_addr = to_v6_mapped(stun_addr);
                            let mut agent = StunAgent::builder(StunTransportType::Udp, local_addr).remote_addr(send_addr).build();
                            let msg = Message::builder_request(BINDING, MessageWriteVec::new()).finish();
                            match agent.send_request(msg, send_addr, stun_now()) {
                                Ok(transmit) => {
                                    let data = transmit.data.as_ref().to_vec();
                                    let to = transmit.to;
                                    drop(transmit);
                                    match socket.send_to(&data, to).await {
                                        Ok(_) => pending.push((agent, stun_addr)),
                                        Err(e) => log::warn!("[ice] Send to {stun_addr}: {e}")
                                    }
                                }
                                Err(e) => log::warn!("[ice] send_request for {stun_addr}: {e:?}")
                            }
                        }
                    }
                }
            }

            let start = Instant::now();
            let mut buf = [0u8; STUN_MAX_PACKET];

            while !pending.is_empty() && start.elapsed() < STUN_TIMEOUT {
                let remaining = STUN_TIMEOUT.saturating_sub(start.elapsed());
                if remaining.is_zero() {
                    break;
                }

                if let Ok(Ok((n, src))) = tokio::time::timeout(remaining, socket.recv_from(&mut buf)).await {
                    if let Ok(msg) = Message::from_bytes(&buf[..n]) {
                        if msg.is_response() {
                            let mut matched = None;
                            for (idx, (agent, _)) in pending.iter_mut().enumerate() {
                                if agent.handle_stun_message(&msg, src) {
                                    matched = Some(idx);
                                    break;
                                }
                            }
                            if let Some(idx) = matched {
                                pending.remove(idx);
                                if let Some(mapped) = extract_mapped_addr(&msg) {
                                    let mut base = local_addr;
                                    if mapped.is_ipv4() && base.is_ipv6() {
                                        base = SocketAddr::new(std::net::Ipv4Addr::UNSPECIFIED.into(), base.port());
                                    } else if mapped.is_ipv6() && base.is_ipv4() {
                                        base = SocketAddr::new(std::net::Ipv6Addr::UNSPECIFIED.into(), base.port());
                                    }
                                    match Candidate::server_reflexive(mapped, base, "udp") {
                                        Ok(c) => {
                                            log::info!("[ice] STUN binding succeeded with {}, candidate: {}", src, c);
                                            candidates.insert(c);
                                        }
                                        Err(e) => log::warn!("[ice] Srflx for {mapped}: {e:?}")
                                    }
                                }
                            }
                        }
                    }
                }

                let now = stun_now();
                pending.retain_mut(|(agent, stun_addr)| match agent.poll(now) {
                    StunAgentPollRet::TransactionTimedOut(_) | StunAgentPollRet::TransactionCancelled(_) => {
                        log::warn!("[ice] STUN timed out for {stun_addr}");
                        false
                    }
                    StunAgentPollRet::WaitUntil(_) => true
                });

                let now = stun_now();
                for (agent, _stun_addr) in pending.iter_mut() {
                    let mut retransmits = Vec::new();
                    while let Some(t) = agent.poll_transmit(now) {
                        retransmits.push((t.data.to_vec(), t.to));
                    }
                    for (data, to) in retransmits {
                        let _ = socket.send_to(&data, to).await;
                    }
                }
            }
        }

        // TURN relay gathering
        let turn_urls = parse_turn_urls(config);
        let turn_info = if !turn_urls.is_empty() {
            let credentials = turn_credentials_from_config(config);

            if let Some(credentials) = credentials {
                let local_addr = socket.local_addr().unwrap_or_else(|_| {
                    SocketAddr::new(std::net::Ipv4Addr::UNSPECIFIED.into(), 0)
                });

                if let Some(first_turn_url) = turn_urls.first() {
                    if let Some(host_port) = turn_url_to_host_port(first_turn_url) {
                        match host_port.to_socket_addrs() {
                            Ok(mut addrs) => {
                                if let Some(turn_server_addr) = addrs.next() {
                                    log::info!("[ice] Attempting TURN allocation to {}", turn_server_addr);
                                    match connect_relay(socket, turn_server_addr, local_addr, credentials).await {
                                        Ok(turn_info) => {
                                            // Add relayed candidate
                                            let relay_addr = turn_info.relay_addr;
                                            log::info!("[ice] TURN allocation succeeded with server {}, relay: {}", turn_server_addr, relay_addr);
                                            match Candidate::relayed(relay_addr, relay_addr, "udp") {
                                                Ok(c) => {
                                                    log::debug!("[ice] Relayed candidate: {}", c);
                                                    candidates.insert(c);
                                                }
                                                Err(e) => {
                                                    log::warn!("[ice] Failed to create relayed candidate for {}: {:?}", relay_addr, e);
                                                }
                                            }
                                            Some(turn_info)
                                        }
                                        Err(e) => {
                                            log::warn!("[ice] TURN allocation failed: {}", e);
                                            None
                                        }
                                    }
                                } else {
                                    log::warn!("[ice] No addresses found for TURN server");
                                    None
                                }
                            }
                            Err(e) => {
                                log::warn!("[ice] Failed to resolve TURN server address: {}", e);
                                None
                            }
                        }
                    } else {
                        log::warn!("[ice] Failed to parse TURN URL: {}", first_turn_url);
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        let result: Vec<Candidate> = candidates.into_iter().collect();
        log::info!("[ice] Gathered {:?} candidates", result);
        Ok((result, turn_info))
    }
}

#[cfg(test)]
mod tests {
    use super::{is_filtered_interface_name, is_windows_virtual_interface_name};

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
            "Hamachi"
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
            "wlan0"
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
            "virbr0"
        ] {
            assert!(is_filtered_interface_name(name, false), "{name}");
            assert!(is_filtered_interface_name(name, true), "{name}");
        }
    }
}
