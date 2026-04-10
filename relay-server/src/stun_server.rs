use anyhow::{anyhow, Context};
use log::{error, info};
use socket2::{Domain, Protocol, Socket, Type};
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};
use stun::message::{Message, BINDING_REQUEST, BINDING_SUCCESS};
use stun::xoraddr::XorMappedAddress;
use tokio::net::UdpSocket;

pub struct BoundStunSockets {
    sockets: Vec<UdpSocket>,
    pub port: u16,
    pub has_ipv4: bool,
    pub has_ipv6: bool
}

impl BoundStunSockets {
    pub fn bind(port: u16) -> anyhow::Result<Self> {
        let ipv4_socket = bind_socket(SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), port), None)
            .with_context(|| format!("failed to bind IPv4 STUN socket on 0.0.0.0:{port}"))?;
        let port = ipv4_socket.local_addr().context("failed to read IPv4 STUN socket address")?.port();

        let mut sockets = vec![ipv4_socket];
        let mut has_ipv6 = false;

        match bind_socket(SocketAddr::new(Ipv6Addr::UNSPECIFIED.into(), port), Some(true)) {
            Ok(socket) => {
                sockets.push(socket);
                has_ipv6 = true;
            }
            Err(error) => {
                log::warn!("IPv6 STUN socket unavailable on [::]:{}: {}", port, error);
            }
        }

        Ok(Self {
            sockets,
            port,
            has_ipv4: true,
            has_ipv6
        })
    }
}

fn bind_socket(addr: SocketAddr, only_v6: Option<bool>) -> anyhow::Result<UdpSocket> {
    let domain = if addr.is_ipv4() { Domain::IPV4 } else { Domain::IPV6 };
    let socket =
        Socket::new(domain, Type::DGRAM, Some(Protocol::UDP)).with_context(|| format!("failed to create UDP socket for {addr}"))?;

    if let Some(only_v6) = only_v6 {
        socket
            .set_only_v6(only_v6)
            .with_context(|| format!("failed to set IPV6_V6ONLY={only_v6} for {addr}"))?;
    }

    socket
        .set_nonblocking(true)
        .with_context(|| format!("failed to set nonblocking UDP socket for {addr}"))?;
    socket.bind(&addr.into()).with_context(|| format!("failed to bind UDP socket on {addr}"))?;

    UdpSocket::from_std(socket.into()).with_context(|| format!("failed to wrap tokio UDP socket for {addr}"))
}

pub async fn run_stun_server(bound: BoundStunSockets) -> anyhow::Result<()> {
    if bound.sockets.is_empty() {
        return Err(anyhow!("no STUN sockets bound"));
    }

    let mut tasks = Vec::with_capacity(bound.sockets.len());
    for socket in bound.sockets {
        tasks.push(tokio::spawn(serve_socket(socket)));
    }

    for task in tasks {
        task.await.context("STUN server task join error")??;
    }

    Ok(())
}

async fn serve_socket(socket: UdpSocket) -> anyhow::Result<()> {
    let addr = socket.local_addr()?;
    info!("STUN server listening on {}", addr);

    let mut buf = [0u8; 1500];
    loop {
        let (len, src_addr) = match socket.recv_from(&mut buf).await {
            Ok(res) => res,
            Err(e) => {
                error!("STUN server recv_from error: {}", e);
                continue;
            }
        };

        let mut message = Message::new();
        if let Err(_e) = message.unmarshal_binary(&buf[..len]) {
            // error!("STUN server unmarshal_binary error: {}", e);
            continue;
        }

        if message.typ == BINDING_REQUEST {
            let mut response = Message::new();
            if let Err(e) = response.build(&[
                Box::new(message.transaction_id),
                Box::new(BINDING_SUCCESS),
                Box::new(XorMappedAddress {
                    ip: src_addr.ip(),
                    port: src_addr.port()
                })
            ]) {
                error!("STUN server build response error: {}", e);
                continue;
            }

            let encoded = match response.marshal_binary() {
                Ok(bytes) => bytes,
                Err(e) => {
                    error!("STUN server marshal_binary error: {}", e);
                    continue;
                }
            };

            if let Err(e) = socket.send_to(&encoded, src_addr).await {
                error!("STUN server send_to error: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{serve_socket, BoundStunSockets};
    use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};
    use std::time::Duration;
    use tokio::net::UdpSocket;

    async fn probe(addr: SocketAddr) -> anyhow::Result<()> {
        let bind_addr = match addr {
            SocketAddr::V4(_) => SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 0),
            SocketAddr::V6(_) => SocketAddr::new(Ipv6Addr::LOCALHOST.into(), 0)
        };

        let socket = UdpSocket::bind(bind_addr).await?;
        socket
            .send_to(
                &[
                    0x00, 0x01, 0x00, 0x00, 0x21, 0x12, 0xA4, 0x42, 0x63, 0x6F, 0x64, 0x65, 0x78, 0x74, 0x65, 0x73, 0x74, 0x31, 0x32,
                    0x33
                ],
                addr
            )
            .await?;

        let mut buf = [0u8; 1500];
        let (len, _) = tokio::time::timeout(Duration::from_secs(1), socket.recv_from(&mut buf)).await??;
        assert!(len >= 20);
        assert_eq!(&buf[..2], &[0x01, 0x01]);

        Ok(())
    }

    #[tokio::test]
    #[ignore = "requires local UDP bind permissions"]
    async fn stun_server_responds_over_ipv4_and_ipv6_when_available() {
        let bound = BoundStunSockets::bind(0).unwrap();
        let port = bound.port;
        let has_ipv6 = bound.has_ipv6;

        let mut tasks = Vec::new();
        for socket in bound.sockets {
            tasks.push(tokio::spawn(serve_socket(socket)));
        }

        probe(SocketAddr::new(Ipv4Addr::LOCALHOST.into(), port)).await.unwrap();

        if has_ipv6 {
            probe(SocketAddr::new(Ipv6Addr::LOCALHOST.into(), port)).await.unwrap();
        }

        for task in tasks {
            task.abort();
            let _ = task.await;
        }
    }
}
