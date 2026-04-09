use std::env;
use std::net::SocketAddr;

use core_services::services::base::Resolve;
use core_services::services::errors::Errors;
use log::error;
use tokio::net::TcpListener as TokioTcpListener;
use tokio::net::UdpSocket;
use tonic::transport::server::TcpIncoming;

pub struct GrpcConnection {
    pub listener: TcpIncoming,
    pub port: u16,
    pub public_host: String // The host of this service that is available for others to connect
}

pub async fn find_grpc_listener(from: Option<u16>) -> Resolve<GrpcConnection> {
    let base_port: u16 = from.unwrap_or(3000); // Starting port
    let max_port: u16 = 50100; // Maximum port to try
    let mut port = base_port;

    let listener = loop {
        let addr: SocketAddr = format!("[::]:{port}").parse().expect("Invalid address");

        match TcpIncoming::bind(addr) {
            Ok(listener) => {
                println!("Server successfully bound to {addr}");
                break listener;
            }
            Err(_) => {
                port += 1;

                if port > max_port {
                    error!(target: "tcp-listener", "No available ports in range {base_port}-{max_port}");
                    return Err(Errors::TcpFailure("No port available".to_owned()));
                }
            }
        }
    };

    Ok(GrpcConnection {
        listener,
        port,
        public_host: env::var("SERVICE_HOST")
            .map(|it| it.parse().expect("SERVICE_HOST must be string"))
            .unwrap_or("host.docker.internal".to_owned())
    })
}

pub struct TcpConnection {
    pub listener: TokioTcpListener,
    pub port: u16,
    pub public_host: String // The host of this service that is available for others to connect
}

pub async fn find_tcp_listener(from: Option<u16>) -> Resolve<TcpConnection> {
    let base_port: u16 = from.unwrap_or(3000); // Starting port
    let max_port: u16 = 50100; // Maximum port to try
    let mut port = base_port;

    let listener = loop {
        let addr: SocketAddr = format!("[::]:{port}").parse().expect("Invalid address");

        match TokioTcpListener::bind(addr).await {
            Ok(listener) => {
                println!("Server successfully bound to {addr}");
                break listener;
            }
            Err(_) => {
                port += 1;

                if port > max_port {
                    error!(
                        target: "tcp-listener",
                        "No available ports in range {base_port}-{max_port}"
                    );
                    return Err(Errors::TcpFailure("No port available".to_owned()));
                }
            }
        }
    };

    Ok(TcpConnection {
        listener,
        port,
        public_host: env::var("SERVICE_HOST").unwrap_or_else(|_| "host.docker.internal".to_owned())
    })
}

/// Bind to a specific port (not finding next available)
/// Returns error if the port is already in use
pub async fn bind_tcp_listener(port: u16) -> Resolve<TcpConnection> {
    let addr: SocketAddr = format!("[::]:{port}").parse().expect("Invalid address");

    let listener = TokioTcpListener::bind(addr).await.map_err(|e| {
        error!(target: "tcp-listener", "Failed to bind to port {}: {}", port, e);
        Errors::TcpFailure(format!("Port {} already in use", port))
    })?;

    println!("Server successfully bound to {addr}");

    Ok(TcpConnection {
        listener,
        port,
        public_host: env::var("SERVICE_HOST").unwrap_or_else(|_| "host.docker.internal".to_owned())
    })
}

pub struct UdpConnection {
    pub socket: UdpSocket,
    pub port: u16,
    pub public_host: String // The host of this service that is available for others to connect
}

pub async fn find_udp_socket(from: Option<u16>) -> Resolve<UdpConnection> {
    let base_port: u16 = from.unwrap_or(3000); // Starting port
    let max_port: u16 = 50100; // Maximum port to try
    let mut port = base_port;

    let socket = loop {
        let addr: SocketAddr = format!("[::]:{port}").parse().expect("Invalid address");

        match UdpSocket::bind(addr).await {
            Ok(socket) => {
                println!("UDP successfully bound to {addr}");
                break socket;
            }
            Err(_) => {
                port += 1;

                if port > max_port {
                    error!(target: "udp-listener", "No available ports in range {base_port}-{max_port}");
                    return Err(Errors::TcpFailure("No port available".to_owned()));
                }
            }
        }
    };

    Ok(UdpConnection {
        socket,
        port,
        public_host: env::var("SERVICE_HOST")
            .map(|it| it.parse().expect("SERVICE_HOST must be string"))
            .unwrap_or("host.docker.internal".to_owned())
    })
}
