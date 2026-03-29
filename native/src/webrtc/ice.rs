use std::collections::HashSet;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use bytecodec::{DecodeExt, EncodeExt};
use str0m::Candidate;
use stun_codec::rfc5389::methods::BINDING;
use stun_codec::rfc5389::Attribute;
use stun_codec::{Message, MessageClass, MessageDecoder, MessageEncoder, TransactionId};
use thiserror::Error;
use tokio::net::{lookup_host, UdpSocket};
use tokio::task::JoinSet;

use schema::devlog::rpc_signalling::server::IceConfig;

const STUN_TIMEOUT: Duration = Duration::from_millis(1500);
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

fn build_binding_request() -> (TransactionId, Vec<u8>) {
    let mut rng_bytes = [0u8; 12];
    rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut rng_bytes);
    let txn_id = TransactionId::new(rng_bytes);

    let message = Message::<Attribute>::new(MessageClass::Request, BINDING, txn_id);
    let bytes = MessageEncoder::new()
        .encode_into_bytes(message)
        .expect("STUN encode never fails for a simple binding request");

    (txn_id, bytes)
}

fn parse_binding_response(buf: &[u8], expected: TransactionId) -> Result<SocketAddr, &'static str> {
    let decoded: Message<Attribute> = MessageDecoder::new()
        .decode_from_bytes(buf)
        .map_err(|_| "decode frame failed")?
        .map_err(|_| "decode message failed")?;

    if decoded.class() != MessageClass::SuccessResponse {
        return Err("class is not SuccessResponse");
    }
    if decoded.transaction_id() != expected {
        return Err("transaction ID mismatch");
    }

    for attr in decoded.attributes() {
        match attr {
            Attribute::XorMappedAddress(xma) => return Ok(xma.address()),
            Attribute::MappedAddress(ma) => return Ok(ma.address()),
            _ => {}
        }
    }
    Err("no MappedAddress or XorMappedAddress found")
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
        IpAddr::V6(v6) => {
            let seg = v6.segments();
            (seg[0] & 0xffc0) != 0xfe80
        }
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

fn stun_url_to_host_port(url: &str) -> Option<String> {
    let stripped = url.strip_prefix("stun:")?;
    if stripped.contains(':') {
        Some(stripped.to_string())
    } else {
        Some(format!("{}:3478", stripped))
    }
}

pub struct IceAgent;

impl IceAgent {
    pub async fn resolve_remote_candidates(sdp: &str) -> String {
        let mut resolved_lines = Vec::new();
        for line in sdp.lines() {
            if line.contains("candidate:") {
                let mut parts: Vec<String> = line.split_whitespace().map(|s| s.to_string()).collect();
                if parts.len() > 5 {
                    let hostname = &parts[4];
                    let resolved_ip = if let Ok(ip) = hostname.parse::<std::net::IpAddr>() {
                        Some(ip)
                    } else {
                        let port = &parts[5];
                        let lookup = format!("{}:{}", hostname, port);
                        match tokio::net::lookup_host(lookup).await {
                            Ok(mut addrs) => addrs.next().map(|a| a.ip()),
                            Err(e) => {
                                log::warn!("[ice] Failed to resolve remote candidate {}: {}", hostname, e);
                                None
                            }
                        }
                    };

                    if let Some(ip) = resolved_ip {
                        let mapped = match ip {
                            std::net::IpAddr::V4(v4) => v4.to_ipv6_mapped().to_string(),
                            std::net::IpAddr::V6(v6) => v6.to_string(),
                        };
                        parts[4] = mapped;
                        resolved_lines.push(parts.join(" "));
                        continue;
                    }
                }
            }
            resolved_lines.push(line.to_string());
        }
        resolved_lines.join("\r\n")
    }

    pub async fn gather_candidates(socket: Arc<UdpSocket>, config: &IceConfig) -> Result<Vec<Candidate>, IceError> {
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
            let mut join_set: JoinSet<Option<(SocketAddr, SocketAddr)>> = JoinSet::new();

            for url_str in &stun_urls {
                let Some(host_port) = stun_url_to_host_port(url_str) else {
                    log::warn!("[ice] Failed to parse STUN URL: {}", url_str);
                    continue;
                };

                let addrs: Vec<SocketAddr> = match lookup_host(&host_port).await {
                    Ok(addrs) => addrs.collect(),
                    Err(e) => {
                        log::warn!("[ice] Failed to resolve STUN server {}: {}", host_port, e);
                        continue;
                    }
                };

                for stun_addr in addrs {
                    let sock = socket.clone();
                    let base_addr = sock.local_addr().ok();
                    let send_addr = to_v6_mapped(stun_addr);

                    join_set.spawn(async move {
                        let (txn_id, request_bytes) = build_binding_request();

                        if let Err(e) = sock.send_to(&request_bytes, send_addr).await {
                            log::warn!("[ice] Failed to send STUN request to {}: {}", stun_addr, e);
                            return None;
                        }

                        let mut buf = [0u8; STUN_MAX_PACKET];
                        match tokio::time::timeout(STUN_TIMEOUT, sock.recv_from(&mut buf)).await {
                            Ok(Ok((n, _src))) => match parse_binding_response(&buf[..n], txn_id) {
                                Ok(mapped) => {
                                    let mut base = base_addr.unwrap_or_else(|| SocketAddr::new(mapped.ip(), mapped.port()));
                                    if mapped.is_ipv4() && base.is_ipv6() {
                                        base = SocketAddr::new(std::net::Ipv4Addr::UNSPECIFIED.into(), base.port());
                                    } else if mapped.is_ipv6() && base.is_ipv4() {
                                        base = SocketAddr::new(std::net::Ipv6Addr::UNSPECIFIED.into(), base.port());
                                    }
                                    Some((mapped, base))
                                }
                                Err(reason) => {
                                    log::warn!("[ice] Invalid STUN response from {}: {}", stun_addr, reason);
                                    None
                                }
                            },
                            Ok(Err(e)) => {
                                log::warn!("[ice] STUN recv error from {}: {}", stun_addr, e);
                                None
                            }
                            Err(_) => {
                                log::warn!("[ice] STUN timeout from {}", stun_addr);
                                None
                            }
                        }
                    });
                }
            }

            while let Some(result) = join_set.join_next().await {
                if let Ok(Some((mapped_addr, base_addr))) = result {
                    match Candidate::server_reflexive(mapped_addr, base_addr, "udp") {
                        Ok(c) => {
                            log::debug!("[ice] Srflx candidate: {} (base: {})", c, base_addr);
                            candidates.insert(c);
                        }
                        Err(e) => {
                            log::warn!("[ice] Failed to create srflx candidate for {}: {:?}", mapped_addr, e);
                        }
                    }
                }
            }
        }

        let result: Vec<Candidate> = candidates.into_iter().collect();
        log::info!("[ice] Gathered {:?} candidates", result);
        Ok(result)
    }
}
