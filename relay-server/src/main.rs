mod config;
mod connection;
mod di;
mod gateway;
mod grpc_middleware;
mod grpc_service;
mod public_ip;
mod stun_server;

use base64::Engine;
use serde::Serialize;
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
use crate::public_ip::{discover_public_addresses, PublicAddresses};

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
    let public_addresses = discover_public_addresses().await.map_err(MainErrors::ExecutionError)?;
    register_relay_once(&registration_url, stun_port, relay_port, &public_addresses)
        .await
        .map_err(MainErrors::ExecutionError)?;
    let di = DiContainer::init(relay_proxy_ipv4(&public_addresses)).await;

    log::info!(
        "Relay Server bound on [::]:{} and advertising public addresses {:?}",
        relay_port,
        public_addresses
    );

    // Start registration loop
    tokio::spawn(async move {
        start_registration_loop(
            stun_port,
            relay_port,
            RegistrationState::new(registration_url, public_addresses)
        )
        .await;
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
    public_ipv4: Option<String>,
    public_ipv6: Option<String>
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RegistrationState {
    registration_url: String,
    public_addresses: PublicAddresses
}

impl RegistrationState {
    fn new(registration_url: String, public_addresses: PublicAddresses) -> Self {
        Self {
            registration_url,
            public_addresses
        }
    }

    fn update_registration_url(&mut self, next_url: String) -> Option<(String, String)> {
        if self.registration_url == next_url {
            return None;
        }

        let previous_url = std::mem::replace(&mut self.registration_url, next_url);
        Some((previous_url, self.registration_url.clone()))
    }
}

fn relay_auth_header() -> String {
    let secret = env::var("RELAY_SERVER_SECRET").unwrap_or_else(|_| "supersecret".to_string());
    let auth_str = format!("user:{}", secret);
    let b64_auth = base64::engine::general_purpose::STANDARD.encode(auth_str);
    format!("Basic {}", b64_auth)
}

fn relay_proxy_ipv4(public_addresses: &PublicAddresses) -> Ipv4Addr {
    public_addresses.ipv4.unwrap_or_else(|| {
        log::warn!("No public IPv4 discovered for relay RTC candidate publication; falling back to 0.0.0.0");
        Ipv4Addr::UNSPECIFIED
    })
}

async fn register_relay_once(url: &str, stun_port: u16, relay_port: u16, public_addresses: &PublicAddresses) -> Result<(), String> {
    let client = reqwest::Client::builder()
        .no_proxy()
        .build()
        .map_err(|error| format!("failed to build relay registration client: {error}"))?;
    let auth_header = relay_auth_header();
    let req_body = RegisterRelayRequest {
        stun_port,
        relay_port,
        public_ipv4: public_addresses.ipv4.map(|ip| ip.to_string()),
        public_ipv6: public_addresses.ipv6.map(|ip| ip.to_string())
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

    Ok(())
}

async fn start_registration_loop(stun_port: u16, relay_port: u16, mut state: RegistrationState) {
    let grpc_gateway = GatewayChannel::new(config::get_gateway_grpc_url(), config::get_gateway_grpc_host());

    let mut interval = tokio::time::interval(Duration::from_secs(5));
    loop {
        interval.tick().await;

        let url = match resolve_registration_url(&grpc_gateway).await {
            Ok(url) => {
                if let Some((previous_url, next_url)) = state.update_registration_url(url.clone()) {
                    log::info!("Relay registration URL changed from {} to {}", previous_url, next_url);
                }

                url
            }
            Err(error) => {
                log::error!("Failed to resolve relay registration URL: {}", error);
                state.registration_url.clone()
            }
        };

        match register_relay_once(&url, stun_port, relay_port, &state.public_addresses).await {
            Ok(()) => {}
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
    use super::{relay_proxy_ipv4, RegistrationState};
    use crate::public_ip::PublicAddresses;
    use std::net::{Ipv4Addr, Ipv6Addr};

    #[test]
    fn relay_proxy_ipv4_uses_discovered_ipv4() {
        let ip = relay_proxy_ipv4(&PublicAddresses {
            ipv4: Some(Ipv4Addr::new(203, 0, 113, 10)),
            ipv6: Some(Ipv6Addr::LOCALHOST)
        });

        assert_eq!(ip, Ipv4Addr::new(203, 0, 113, 10));
    }

    #[test]
    fn relay_proxy_ipv4_falls_back_to_unspecified() {
        let ip = relay_proxy_ipv4(&PublicAddresses {
            ipv4: None,
            ipv6: Some(Ipv6Addr::LOCALHOST)
        });

        assert_eq!(ip, Ipv4Addr::UNSPECIFIED);
    }

    #[test]
    fn registration_state_tracks_registration_url_changes() {
        let mut state = RegistrationState::new(
            "https://gateway.example/rpc-signalling-local/register-relay".to_string(),
            PublicAddresses {
                ipv4: Some(Ipv4Addr::new(203, 0, 113, 10)),
                ipv6: Some(Ipv6Addr::LOCALHOST)
            }
        );

        let changed = state.update_registration_url("https://gateway.example/rpc-signalling-europe/register-relay".to_string());

        assert_eq!(
            changed,
            Some((
                "https://gateway.example/rpc-signalling-local/register-relay".to_string(),
                "https://gateway.example/rpc-signalling-europe/register-relay".to_string()
            ))
        );
    }

    #[test]
    fn registration_state_ignores_unchanged_values() {
        let mut state = RegistrationState::new(
            "https://gateway.example/rpc-signalling-local/register-relay".to_string(),
            PublicAddresses {
                ipv4: Some(Ipv4Addr::new(203, 0, 113, 10)),
                ipv6: Some(Ipv6Addr::LOCALHOST)
            }
        );

        assert_eq!(
            state.update_registration_url("https://gateway.example/rpc-signalling-local/register-relay".to_string()),
            None
        );
    }
}
