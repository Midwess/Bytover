use std::collections::HashSet;
use std::net::{IpAddr, SocketAddr, ToSocketAddrs};
use std::time::{Duration, Instant};

use str0m::Candidate;
use stun_proto::agent::{StunAgent, StunAgentPollRet};
use stun_proto::types::attribute::{AttributeType, MappedSocketAddr, XorMappedAddress};
use stun_proto::types::message::{Message, MessageWriteVec, BINDING};
use stun_proto::types::prelude::*;
use stun_proto::types::TransportType;
use stun_proto::Instant as StunInstant;
use thiserror::Error;

use schema::devlog::rpc_signalling::server::IceConfig;

const STUN_TIMEOUT: Duration = Duration::from_millis(3000);
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
    let name = &iface.name;
    if name.starts_with("docker") ||
        name.starts_with("vbox") ||
        name.starts_with("br-") ||
        name.starts_with("veth") ||
        name.starts_with("virbr")
    {
        return false;
    }
    match iface.ip() {
        IpAddr::V4(v4) => !v4.is_link_local(),
        IpAddr::V6(v6) => (v6.segments()[0] & 0xffc0) != 0xfe80
    }
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

fn extract_mapped_addr(msg: &Message<'_>) -> Option<SocketAddr> {
    if let Ok(xma) = msg.attribute::<XorMappedAddress>() {
        return Some(xma.addr(msg.transaction_id()));
    }
    msg.raw_attribute(AttributeType::new(0x0001))
        .and_then(|raw| MappedSocketAddr::from_raw(&raw).ok())
        .map(|m| m.addr())
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

    pub async fn gather_candidates(socket: &tokio::net::UdpSocket, config: &IceConfig) -> Result<Vec<Candidate>, IceError> {
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
                            let mut agent = StunAgent::builder(TransportType::Udp, local_addr).remote_addr(send_addr).build();
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
                                            log::debug!("[ice] Srflx candidate: {} (base: {})", c, base);
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

        let result: Vec<Candidate> = candidates.into_iter().collect();
        log::info!("[ice] Gathered {:?} candidates", result);
        Ok(result)
    }
}
