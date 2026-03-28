use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use str0m::{Candidate, Rtc};
use thiserror::Error;
use tokio::net::UdpSocket;
use tokio::sync::Semaphore;

use schema::devlog::rpc_signalling::server::IceConfig;

#[derive(Debug, Error)]
pub enum IceError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("STUN timeout")]
    Timeout,

    #[error("STUN response too short")]
    ResponseTooShort,

    #[error("unexpected STUN response type: {0:#06x}")]
    UnexpectedResponseType(u16),

    #[error("STUN transaction ID mismatch")]
    TransactionIdMismatch,

    #[error("no XOR-MAPPED-ADDRESS in STUN response")]
    NoMappedAddress,
}

/// Shared ICE candidate gatherer.
///
/// Cloning an `IceAgent` shares the same underlying config and semaphore, so all
/// clones contend on the same queue — only one `gather_candidates` call runs at a
/// time across the entire process.
#[derive(Clone)]
pub struct IceAgent(Arc<IceAgentInner>);

struct IceAgentInner {
    config: IceConfig,
    /// Permits = 1: serialises concurrent gather attempts.
    semaphore: Semaphore,
}

impl IceAgent {
    pub fn new(config: IceConfig) -> Self {
        Self(Arc::new(IceAgentInner {
            config,
            semaphore: Semaphore::new(1),
        }))
    }

    /// Gather server-reflexive candidates for `rtc`.
    ///
    /// If another caller is already gathering, this call waits until that
    /// gather completes before starting its own.
    pub async fn gather_candidates(&self, rtc: &mut Rtc, local_addr: SocketAddr) {
        let _permit = self
            .0
            .semaphore
            .acquire()
            .await
            .expect("IceAgent semaphore closed");
        gather_srflx_candidates(rtc, local_addr, &self.0.config).await;
    }
}

async fn gather_srflx_candidates(rtc: &mut Rtc, local_addr: SocketAddr, config: &IceConfig) {
    for url in &config.urls {
        let Some(stun_addr) = parse_stun_url(url) else {
            continue;
        };

        match stun_binding(local_addr, stun_addr).await {
            Ok(srflx_addr) => {
                match Candidate::server_reflexive(srflx_addr, local_addr, "udp") {
                    Ok(candidate) => {
                        rtc.add_local_candidate(candidate);
                        log::info!(
                            "[webrtc-client] Added srflx candidate: {srflx_addr} (via {url})"
                        );
                    }
                    Err(e) => {
                        log::warn!("[webrtc-client] Failed to create srflx candidate: {e}");
                    }
                }
            }
            Err(e) => {
                log::warn!("[webrtc-client] STUN binding to {url} failed: {e}");
            }
        }
    }
}

fn parse_stun_url(url: &str) -> Option<SocketAddr> {
    let host_port = url
        .strip_prefix("stun:")
        .or_else(|| url.strip_prefix("stuns:"))?;

    let host_port = host_port.split('?').next().unwrap_or(host_port);

    if let Ok(addr) = host_port.parse::<SocketAddr>() {
        return Some(addr);
    }

    let (host, port) = if let Some((h, p)) = host_port.rsplit_once(':') {
        (h, p.parse::<u16>().ok()?)
    } else {
        (host_port, 3478)
    };

    use std::net::ToSocketAddrs;
    format!("{host}:{port}").to_socket_addrs().ok()?.next()
}

async fn stun_binding(
    _local_addr: SocketAddr,
    stun_server: SocketAddr,
) -> Result<SocketAddr, IceError> {
    let socket = UdpSocket::bind("0.0.0.0:0").await?;

    let tid: [u8; 12] = rand::random::<[u8; 12]>();

    let mut request = vec![0u8; 20];
    request[0] = 0x00;
    request[1] = 0x01;
    request[2] = 0x00;
    request[3] = 0x00;
    request[4..8].copy_from_slice(&0x2112A442u32.to_be_bytes());
    request[8..20].copy_from_slice(&tid);

    socket.send_to(&request, stun_server).await?;

    let mut buf = [0u8; 256];
    let n = tokio::time::timeout(Duration::from_secs(3), socket.recv(&mut buf))
        .await
        .map_err(|_| IceError::Timeout)??;

    if n < 20 {
        return Err(IceError::ResponseTooShort);
    }

    let msg_type = u16::from_be_bytes([buf[0], buf[1]]);
    if msg_type != 0x0101 {
        return Err(IceError::UnexpectedResponseType(msg_type));
    }

    let resp_tid = &buf[8..20];
    if resp_tid != tid {
        return Err(IceError::TransactionIdMismatch);
    }

    let msg_len = u16::from_be_bytes([buf[2], buf[3]]) as usize;
    let attrs = &buf[20..20 + msg_len.min(n - 20)];
    parse_xor_mapped_address(attrs)
}

fn parse_xor_mapped_address(mut attrs: &[u8]) -> Result<SocketAddr, IceError> {
    const MAGIC: u32 = 0x2112A442;

    while attrs.len() >= 4 {
        let attr_type = u16::from_be_bytes([attrs[0], attrs[1]]);
        let attr_len = u16::from_be_bytes([attrs[2], attrs[3]]) as usize;

        if attrs.len() < 4 + attr_len {
            break;
        }

        let value = &attrs[4..4 + attr_len];

        if attr_type == 0x0020 && value.len() >= 8 {
            let family = value[1];
            let xport = u16::from_be_bytes([value[2], value[3]]) ^ (MAGIC >> 16) as u16;

            if family == 0x01 && value.len() >= 8 {
                let xaddr =
                    u32::from_be_bytes([value[4], value[5], value[6], value[7]]) ^ MAGIC;
                let ip = std::net::Ipv4Addr::from(xaddr);
                return Ok(SocketAddr::new(ip.into(), xport));
            }
        }

        let padded = (attr_len + 3) & !3;
        attrs = &attrs[4 + padded..];
    }

    Err(IceError::NoMappedAddress)
}
