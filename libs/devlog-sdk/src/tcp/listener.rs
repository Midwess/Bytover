use std::env;
use std::net::{SocketAddr, TcpListener as StdTcpListener};

use core_services::services::base::Resolve;
use core_services::services::errors::Errors;
use log::error;
use socket2::{Domain, Protocol, Socket, Type};
use tokio::net::{TcpListener as TokioTcpListener, UdpSocket};
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

        let socket = match Socket::new(Domain::IPV6, Type::STREAM, Some(Protocol::TCP)) {
            Ok(socket) => socket,
            Err(error) => {
                error!(target: "tcp-listener", "Failed to create TCP socket: {}", error);
                return Err(Errors::TcpFailure(format!("Failed to create TCP socket: {error}")));
            }
        };

        if let Err(error) = socket.set_only_v6(false) {
            error!(target: "tcp-listener", "Failed to enable dual-stack TCP socket: {}", error);
            return Err(Errors::TcpFailure(format!("Failed to enable dual-stack TCP socket: {error}")));
        }

        if let Err(error) = socket.set_nonblocking(true) {
            error!(target: "tcp-listener", "Failed to set TCP socket nonblocking: {}", error);
            return Err(Errors::TcpFailure(format!("Failed to set TCP socket nonblocking: {error}")));
        }

        match socket.bind(&addr.into()) {
            Ok(()) => {
                if let Err(error) = socket.listen(512) {
                    error!(target: "tcp-listener", "Failed to listen on TCP socket: {}", error);
                    return Err(Errors::TcpFailure(format!("Failed to listen on TCP socket: {error}")));
                }
                // Convert socket2::Socket -> StdTcpListener -> TokioTcpListener -> TcpIncoming
                let std_listener: StdTcpListener = socket.into();
                match TokioTcpListener::from_std(std_listener) {
                    Ok(tokio_listener) => {
                        println!("gRPC server successfully bound to {addr}");
                        break TcpIncoming::from(tokio_listener);
                    }
                    Err(error) => {
                        error!(target: "tcp-listener", "Failed to create TokioTcpListener: {}", error);
                        return Err(Errors::TcpFailure(format!("Failed to create TokioTcpListener: {error}")));
                    }
                }
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

        let socket = match Socket::new(Domain::IPV6, Type::STREAM, Some(Protocol::TCP)) {
            Ok(socket) => socket,
            Err(error) => {
                error!(target: "tcp-listener", "Failed to create TCP socket: {}", error);
                return Err(Errors::TcpFailure(format!("Failed to create TCP socket: {error}")));
            }
        };

        if let Err(error) = socket.set_only_v6(false) {
            error!(target: "tcp-listener", "Failed to enable dual-stack TCP socket: {}", error);
            return Err(Errors::TcpFailure(format!("Failed to enable dual-stack TCP socket: {error}")));
        }

        if let Err(error) = socket.set_nonblocking(true) {
            error!(target: "tcp-listener", "Failed to set TCP socket nonblocking: {}", error);
            return Err(Errors::TcpFailure(format!("Failed to set TCP socket nonblocking: {error}")));
        }

        match socket.bind(&addr.into()) {
            Ok(()) => {
                if let Err(error) = socket.listen(512) {
                    error!(target: "tcp-listener", "Failed to listen on TCP socket: {}", error);
                    return Err(Errors::TcpFailure(format!("Failed to listen on TCP socket: {error}")));
                }
                match TokioTcpListener::from_std(socket.into()) {
                    Ok(listener) => {
                        println!("TCP server successfully bound to {addr}");
                        break listener;
                    }
                    Err(error) => {
                        error!(target: "tcp-listener", "Failed to create TokioTcpListener: {}", error);
                        return Err(Errors::TcpFailure(format!("Failed to create TokioTcpListener: {error}")));
                    }
                }
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

    let socket = Socket::new(Domain::IPV6, Type::STREAM, Some(Protocol::TCP)).map_err(|e| {
        error!(target: "tcp-listener", "Failed to create TCP socket: {}", e);
        Errors::TcpFailure(format!("Failed to create TCP socket: {e}"))
    })?;

    if let Err(error) = socket.set_only_v6(false) {
        error!(target: "tcp-listener", "Failed to enable dual-stack TCP socket: {}", error);
        return Err(Errors::TcpFailure(format!("Failed to enable dual-stack TCP socket: {error}")));
    }

    socket.bind(&addr.into()).map_err(|e| {
        error!(target: "tcp-listener", "Failed to bind to port {}: {}", port, e);
        Errors::TcpFailure(format!("Port {} already in use", port))
    })?;

    if let Err(error) = socket.listen(512) {
        error!(target: "tcp-listener", "Failed to listen on TCP socket: {}", error);
        return Err(Errors::TcpFailure(format!("Failed to listen on TCP socket: {error}")));
    }

    let listener = TokioTcpListener::from_std(socket.into()).map_err(|e| {
        error!(target: "tcp-listener", "Failed to wrap TCP socket: {}", e);
        Errors::TcpFailure(format!("Failed to wrap TCP socket: {e}"))
    })?;

    println!("TCP server successfully bound to {addr}");

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

        let socket = match Socket::new(Domain::IPV6, Type::DGRAM, Some(Protocol::UDP)) {
            Ok(socket) => socket,
            Err(error) => {
                error!(target: "udp-listener", "Failed to create UDP socket: {}", error);
                return Err(Errors::TcpFailure(format!("Failed to create UDP socket: {error}")));
            }
        };

        if let Err(error) = socket.set_only_v6(false) {
            error!(target: "udp-listener", "Failed to enable dual-stack UDP socket: {}", error);
            return Err(Errors::TcpFailure(format!("Failed to enable dual-stack UDP socket: {error}")));
        }

        if let Err(error) = socket.set_nonblocking(true) {
            error!(target: "udp-listener", "Failed to set UDP socket nonblocking: {}", error);
            return Err(Errors::TcpFailure(format!("Failed to set UDP socket nonblocking: {error}")));
        }

        match socket.bind(&addr.into()) {
            Ok(()) => {
                println!("UDP successfully bound to {addr}");
                match UdpSocket::from_std(socket.into()) {
                    Ok(socket) => break socket,
                    Err(error) => {
                        error!(target: "udp-listener", "Failed to wrap UDP socket: {}", error);
                        return Err(Errors::TcpFailure(format!("Failed to wrap UDP socket: {error}")));
                    }
                }
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
