mod config;
mod connection;
mod di;
mod gateway;
mod grpc_middleware;
mod grpc_service;
mod stun_server;

use base64::Engine;
use serde::{Deserialize, Serialize};
use std::net::Ipv4Addr;
use std::time::Duration;

use devlog_sdk::tcp::listener::{find_grpc_listener, find_udp_socket};
use schema::devlog::bitbridge::p2p_orchestration_service_client::P2pOrchestrationServiceClient;
use schema::devlog::bitbridge::relay_service_server::RelayServiceServer;
use std::env;
use tonic::transport::Server;
use tonic::Request;
use tonic_middleware::InterceptorFor;

use crate::di::DiContainer;
use crate::gateway::GatewayChannel;
use crate::grpc_service::RelayServiceImpl;

#[derive(thiserror::Error, Debug)]
enum MainErrors {
    #[error("Transport error {0}")]
    TransportError(#[from] tonic::transport::Error),
    #[error("DI container error {0}")]
    DiContainerError(String),
    #[error("Execution error {0}")]
    ExecutionError(String)
}

fn main() -> Result<(), MainErrors> {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    // 32 MB guard for deep RTC/DTLS/SCTP poll paths.
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_stack_size(32 * 1024 * 1024)
        .build()
        .expect("Failed to build tokio runtime")
        .block_on(async_main())
}

async fn async_main() -> Result<(), MainErrors> {
    log::info!("Starting relay server...");

    // Prepare STUN server
    let stun_port = 3478;
    let udp_conn = find_udp_socket(Some(stun_port)).await.map_err(|e| MainErrors::DiContainerError(e.to_string()))?;

    // Prepare gRPC server
    let connection = find_grpc_listener(Some(9101)).await.map_err(|e| MainErrors::DiContainerError(e.to_string()))?;
    let relay_port = connection.port;
    let grpc_gateway = GatewayChannel::new(config::get_gateway_grpc_url(), config::get_gateway_grpc_host());
    let registration_url = resolve_registration_url(&grpc_gateway).await.map_err(MainErrors::ExecutionError)?;
    let public_ip = register_relay_once(&registration_url, stun_port, relay_port)
        .await
        .map_err(MainErrors::ExecutionError)?;
    let di = DiContainer::init(public_ip).await;

    log::info!(
        "Relay Server bound on [::]:{} and advertising {}:{}",
        relay_port,
        public_ip,
        relay_port
    );

    // Start registration loop
    tokio::spawn(async move {
        start_registration_loop(stun_port, relay_port, public_ip).await;
    });

    let grpc_server = Server::builder()
        .add_service(InterceptorFor::new(
            RelayServiceServer::new(RelayServiceImpl::new(di.proxy_manager.clone())),
            di.get_auth_middleware()
        ))
        .serve_with_incoming(connection.listener);

    let stun_server = stun_server::run_stun_server(udp_conn);

    tokio::select! {
        res = grpc_server => {
            if let Err(e) = res {
                log::error!("gRPC server exited with error: {:?}", e);
                return Err(MainErrors::TransportError(e));
            }
            log::info!("gRPC server stopped");
        },
        res = stun_server => {
            if let Err(e) = res {
                log::error!("STUN server exited with error: {:?}", e);
                return Err(MainErrors::ExecutionError(e.to_string()));
            }
            log::info!("STUN server stopped");
        }
    }

    Ok(())
}

#[derive(Serialize)]
struct RegisterRelayRequest {
    stun_port: u16,
    relay_port: u16,
    relay_host: String,
    public_ip: Option<String>
}

#[derive(Debug, Deserialize)]
struct RegisterRelayResponse {
    ip_address: String
}

fn relay_auth_header() -> String {
    let secret = env::var("RELAY_SERVER_SECRET").unwrap_or_else(|_| "supersecret".to_string());
    let auth_str = format!("user:{}", secret);
    let b64_auth = base64::engine::general_purpose::STANDARD.encode(auth_str);
    format!("Basic {}", b64_auth)
}

fn parse_registered_ipv4(ip_address: &str) -> Result<Ipv4Addr, String> {
    ip_address
        .trim()
        .parse::<Ipv4Addr>()
        .map_err(|error| format!("signalling returned non-IPv4 address `{}`: {}", ip_address, error))
}

async fn register_relay_once(url: &str, stun_port: u16, relay_port: u16) -> Result<Ipv4Addr, String> {
    let client = reqwest::Client::builder()
        .no_proxy()
        .build()
        .map_err(|error| format!("failed to build relay registration client: {error}"))?;
    let auth_header = relay_auth_header();
    let req_body = RegisterRelayRequest {
        stun_port,
        relay_port,
        relay_host: config::get_relay_control_host(),
        public_ip: config::get_relay_public_ip()
    };

    let response = client
        .post(url)
        .header("authorization", &auth_header)
        .json(&req_body)
        .send()
        .await
        .map_err(|error| format!("failed to register relay: {error}"))?;

    if !response.status().is_success() {
        return Err(format!("failed to register relay: status {}", response.status()));
    }

    let body: RegisterRelayResponse = response
        .json()
        .await
        .map_err(|error| format!("failed to decode relay registration response: {error}"))?;

    parse_registered_ipv4(&body.ip_address)
}

async fn start_registration_loop(stun_port: u16, relay_port: u16, public_ip: Ipv4Addr) {
    let grpc_gateway = GatewayChannel::new(config::get_gateway_grpc_url(), config::get_gateway_grpc_host());

    let mut registration_url = None::<String>;

    let mut interval = tokio::time::interval(Duration::from_secs(5));
    loop {
        interval.tick().await;

        let url = match resolve_registration_url(&grpc_gateway).await {
            Ok(url) => {
                if registration_url.as_deref() != Some(url.as_str()) {
                    log::info!("Resolved relay registration URL to {}", url);
                    registration_url = Some(url.clone());
                }

                url
            }
            Err(error) => {
                log::error!("Failed to resolve relay registration URL: {}", error);
                let Some(url) = registration_url.clone() else {
                    continue;
                };
                url
            }
        };

        match register_relay_once(&url, stun_port, relay_port).await {
            Ok(ip) => {
                if ip != public_ip {
                    log::error!(
                        "Relay registration returned unexpected IPv4 {} after startup cached {}",
                        ip,
                        public_ip
                    );
                }
            }
            Err(error) => {
                log::error!("Failed to register relay heartbeat: {}", error);
            }
        }
    }
}

async fn resolve_registration_url(grpc_gateway: &GatewayChannel) -> Result<String, String> {
    let channel = grpc_gateway.connect().await?;

    let mut client = P2pOrchestrationServiceClient::new(channel);
    let response = client
        .get_region(Request::new(schema::devlog::bitbridge::GetRegionRequest {}))
        .await
        .map_err(|error| format!("backend get_region failed: {error}"))?
        .into_inner();

    Ok(config::get_signalling_registration_url(&response.signalling_route))
}

#[cfg(test)]
mod tests {
    use super::parse_registered_ipv4;
    use std::net::Ipv4Addr;

    #[test]
    fn parse_registered_ipv4_accepts_ipv4() {
        let ip = parse_registered_ipv4("203.0.113.10").unwrap();
        assert_eq!(ip, Ipv4Addr::new(203, 0, 113, 10));
    }

    #[test]
    fn parse_registered_ipv4_rejects_ipv6() {
        let error = parse_registered_ipv4("2001:db8::1").unwrap_err();
        assert!(error.contains("non-IPv4"));
    }
}
