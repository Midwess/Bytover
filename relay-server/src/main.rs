mod config;
mod gateway;
mod public_ip;

use base64::Engine;
use serde::Serialize;
use std::time::Duration;

use devlog_sdk::tcp::listener::find_grpc_listener;
use schema::devlog::bitbridge::p2p_orchestration_service_client::P2pOrchestrationServiceClient;
use std::env;
use tonic::Request;

use crate::gateway::GatewayChannel;
use crate::public_ip::{discover_public_addresses, PublicAddresses};
use turn_server::config::{Auth, Config, Interface, Server as TurnServerConfig};

#[derive(thiserror::Error, Debug)]
enum MainErrors {
    #[error("Execution error {0}")]
    ExecutionError(String)
}

fn main() -> Result<(), MainErrors> {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_stack_size(32 * 1024 * 1024)
        .build()
        .expect("Failed to build tokio runtime")
        .block_on(async_main())
}

async fn async_main() -> Result<(), MainErrors> {
    log::info!("Starting relay server...");

    let requested_turn_port = 19101;
    let public_addresses = discover_public_addresses()
        .await
        .map_err(MainErrors::ExecutionError)?
        .retain_families(true, true);

    let turn_external_addr = format!(
        "{}:{}",
        public_addresses.ipv4.map(|ip| ip.to_string()).unwrap_or_else(|| "0.0.0.0".to_string()),
        requested_turn_port
    ).parse().map_err(|e| MainErrors::ExecutionError(format!("invalid turn external address: {e}")))?;

    let turn_config = Config {
        server: TurnServerConfig {
            interfaces: vec![Interface::Udp {
                listen: format!("0.0.0.0:{}", requested_turn_port).parse().unwrap(),
                external: turn_external_addr,
                idle_timeout: 20,
                mtu: 1500,
            }],
            ..Default::default()
        },
        auth: Auth {
            static_credentials: Default::default(),
            static_auth_secret: Some(env::var("TURN_AUTH_SECRET").unwrap_or_else(|_| "relay-secret".to_string())),
            enable_hooks_auth: false,
        },
        ..Default::default()
    };

    let _listener = find_grpc_listener(Some(9101)).await.map_err(|e| MainErrors::ExecutionError(e.to_string()))?;
    let grpc_gateway = GatewayChannel::new(config::get_gateway_grpc_url(), config::get_gateway_grpc_host());
    let registration_url = resolve_registration_url(&grpc_gateway).await.map_err(MainErrors::ExecutionError)?;

    let turn_port = requested_turn_port;
    register_relay_once(&registration_url, 3478, turn_port, &public_addresses)
        .await
        .map_err(MainErrors::ExecutionError)?;

    log::info!(
        "Relay Server advertising public addresses {:?}, STUN=3478, TURN={}",
        public_addresses,
        turn_port
    );

    tokio::spawn(async move {
        start_registration_loop(
            3478,
            turn_port,
            RegistrationState::new(registration_url, public_addresses)
        )
        .await;
    });

    let turn_handle = tokio::spawn(async move {
        if let Err(e) = turn_server::start_server(turn_config).await {
            log::error!("TURN server exited with error: {:?}", e);
        }
    });

    tokio::select! {
        res = turn_handle => {
            if let Err(e) = res {
                log::error!("TURN server task panicked: {:?}", e);
                return Err(MainErrors::ExecutionError(format!("TURN server panicked: {e:?}")));
            }
            log::info!("TURN server stopped");
        }
    }

    Ok(())
}

#[derive(Serialize)]
struct RegisterRelayRequest {
    stun_port: u16,
    relay_port: u16,
    public_ipv4: Option<String>,
    public_ipv6: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RegistrationState {
    registration_url: String,
    public_addresses: PublicAddresses
}

impl RegistrationState {
    fn new(registration_url: String, public_addresses: PublicAddresses) -> Self {
        Self { registration_url, public_addresses }
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
        public_ipv6: public_addresses.ipv6.map(|ip| ip.to_string()),
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
    use super::RegistrationState;
    use crate::public_ip::PublicAddresses;
    use std::net::{Ipv4Addr, Ipv6Addr};

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
