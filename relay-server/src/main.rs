mod config;
mod gateway;
mod public_ip;

use base64::Engine;
use serde::Serialize;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::Duration;

use schema::devlog::bitbridge::p2p_orchestration_service_client::P2pOrchestrationServiceClient;
use std::env;
use tonic::Request;

use crate::gateway::GatewayChannel;
use crate::public_ip::{discover_public_addresses, PublicAddresses};
use turn_server::config::{Auth, Config, Interface, Server as TurnServerConfig};

#[derive(thiserror::Error, Debug)]
enum MainErrors {
    #[error("Execution error {0}")]
    ExecutionError(String),
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

    let turn_port = env::var("TURN_PORT").unwrap_or_else(|_| "19101".to_string()).parse().unwrap();
    let public_addresses = discover_public_addresses().await.map_err(MainErrors::ExecutionError)?.retain_families(true, true);

    let turn_external_addr = turn_external_addr(&public_addresses, turn_port).map_err(MainErrors::ExecutionError)?;

    let turn_username = env::var("TURN_USERNAME").unwrap_or_else(|_| "relay".to_string());
    let turn_password = env::var("TURN_PASSWORD").unwrap_or_else(|_| "relay-secret".to_string());

    let turn_config = Config {
        server: TurnServerConfig {
            interfaces: vec![Interface::Udp {
                listen: format!("[::]:{}", turn_port).parse().unwrap(),
                external: turn_external_addr,
                idle_timeout: 20,
                mtu: 1500,
            }],
            ..Default::default()
        },
        auth: Auth {
            static_credentials: {
                let mut creds = HashMap::new();
                let user = turn_username.clone();
                let pass = turn_password.clone();
                creds.insert(user, pass);
                creds
            },
            static_auth_secret: Some(env::var("TURN_AUTH_SECRET").unwrap_or_else(|_| "relay-secret".to_string())),
            enable_hooks_auth: false,
        },
        ..Default::default()
    };

    log::info!(
        "Relay Server serving public addresses {:?}, STUN/TURN={}",
        public_addresses,
        turn_port
    );

    let registration_state = RegistrationState::new(public_addresses, turn_username.clone(), turn_password.clone());

    let turn_handle = tokio::spawn(async move {
        if let Err(e) = turn_server::start_server(turn_config).await {
            log::error!("TURN server exited with error: {:?}", e);
        }
    });

    tokio::spawn(async move {
        start_registration_loop(turn_port, turn_port, turn_port, registration_state).await;
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

fn turn_external_addr(public_addresses: &PublicAddresses, port: u16) -> Result<SocketAddr, String> {
    match (public_addresses.ipv4, public_addresses.ipv6) {
        (Some(ipv4), _) => Ok(SocketAddr::new(ipv4.into(), port)),
        (None, Some(ipv6)) => Ok(SocketAddr::new(ipv6.into(), port)),
        (None, None) => Err("relay registration requires at least one public IP address".to_string()),
    }
}

#[derive(Serialize)]
struct RegisterRelayRequest {
    stun_port: u16,
    relay_port: u16,
    turn_port: u16,
    public_ipv4: Option<String>,
    public_ipv6: Option<String>,
    turn_username: String,
    turn_password: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RegistrationState {
    registration_url: Option<String>,
    public_addresses: PublicAddresses,
    turn_username: String,
    turn_password: String,
}

impl RegistrationState {
    fn new(public_addresses: PublicAddresses, turn_username: String, turn_password: String) -> Self {
        Self {
            registration_url: None,
            public_addresses,
            turn_username,
            turn_password,
        }
    }

    fn update_registration_url(&mut self, next_url: String) -> Option<(Option<String>, String)> {
        if self.registration_url.as_deref() == Some(next_url.as_str()) {
            return None;
        }
        let previous_url = self.registration_url.replace(next_url);
        Some((
            previous_url,
            self.registration_url.clone().expect("registration url was just set"),
        ))
    }
}

fn relay_auth_header() -> String {
    let secret = env::var("RELAY_SERVER_SECRET").unwrap_or_else(|_| "supersecret".to_string());
    let auth_str = format!("user:{}", secret);
    let b64_auth = base64::engine::general_purpose::STANDARD.encode(auth_str);
    format!("Basic {}", b64_auth)
}

async fn register_relay_once(
    url: &str,
    stun_port: u16,
    relay_port: u16,
    turn_port: u16,
    turn_username: &str,
    turn_password: &str,
    public_addresses: &PublicAddresses,
) -> Result<(), String> {
    let client = reqwest::Client::builder()
        .no_proxy()
        .build()
        .map_err(|error| format!("failed to build relay registration client: {error}"))?;
    let auth_header = relay_auth_header();
    let req_body = RegisterRelayRequest {
        stun_port,
        relay_port,
        turn_port,
        public_ipv4: public_addresses.ipv4.map(|ip| ip.to_string()),
        public_ipv6: public_addresses.ipv6.map(|ip| ip.to_string()),
        turn_username: turn_username.to_string(),
        turn_password: turn_password.to_string(),
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

async fn start_registration_loop(stun_port: u16, relay_port: u16, turn_port: u16, mut state: RegistrationState) {
    let grpc_gateway = GatewayChannel::new(config::get_gateway_grpc_url(), config::get_gateway_grpc_host());

    let mut interval = tokio::time::interval(Duration::from_secs(5));
    loop {
        interval.tick().await;

        let url = match resolve_registration_url(&grpc_gateway).await {
            Ok(url) => {
                if let Some((previous_url, next_url)) = state.update_registration_url(url.clone()) {
                    if let Some(previous_url) = previous_url {
                        log::info!("Relay registration URL changed from {} to {}", previous_url, next_url);
                    } else {
                        log::info!("Relay registration URL resolved to {}", next_url);
                    }
                }
                Some(url)
            }
            Err(error) => {
                log::error!("Failed to resolve relay registration URL: {}", error);
                state.registration_url.clone()
            }
        };

        let Some(url) = url else {
            continue;
        };

        match register_relay_once(
            &url,
            stun_port,
            relay_port,
            turn_port,
            &state.turn_username,
            &state.turn_password,
            &state.public_addresses,
        )
        .await
        {
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
    use super::{turn_external_addr, RegistrationState};
    use crate::public_ip::PublicAddresses;
    use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};

    #[test]
    fn registration_state_tracks_initial_resolution() {
        let mut state = RegistrationState::new(
            PublicAddresses {
                ipv4: Some(Ipv4Addr::new(203, 0, 113, 10)),
                ipv6: Some(Ipv6Addr::LOCALHOST),
            },
            "user".to_string(),
            "pass".to_string(),
        );

        let changed = state.update_registration_url("https://gateway.example/rpc-signalling-local/register-relay".to_string());

        assert_eq!(
            changed,
            Some((None, "https://gateway.example/rpc-signalling-local/register-relay".to_string()))
        );
    }

    #[test]
    fn registration_state_tracks_registration_url_changes() {
        let mut state = RegistrationState::new(
            PublicAddresses {
                ipv4: Some(Ipv4Addr::new(203, 0, 113, 10)),
                ipv6: Some(Ipv6Addr::LOCALHOST),
            },
            "user".to_string(),
            "pass".to_string(),
        );

        state.update_registration_url("https://gateway.example/rpc-signalling-local/register-relay".to_string());

        assert_eq!(
            state.update_registration_url("https://gateway.example/rpc-signalling-europe/register-relay".to_string()),
            Some((
                Some("https://gateway.example/rpc-signalling-local/register-relay".to_string()),
                "https://gateway.example/rpc-signalling-europe/register-relay".to_string()
            ))
        );
    }

    #[test]
    fn registration_state_ignores_unchanged_values() {
        let mut state = RegistrationState::new(
            PublicAddresses {
                ipv4: Some(Ipv4Addr::new(203, 0, 113, 10)),
                ipv6: Some(Ipv6Addr::LOCALHOST),
            },
            "user".to_string(),
            "pass".to_string(),
        );

        state.update_registration_url("https://gateway.example/rpc-signalling-local/register-relay".to_string());

        assert_eq!(
            state.update_registration_url("https://gateway.example/rpc-signalling-local/register-relay".to_string()),
            None
        );
    }

    #[test]
    fn turn_external_addr_prefers_ipv4_when_available() {
        let addr = turn_external_addr(
            &PublicAddresses {
                ipv4: Some(Ipv4Addr::new(203, 0, 113, 10)),
                ipv6: Some(Ipv6Addr::LOCALHOST),
            },
            19101,
        )
        .unwrap();

        assert_eq!(addr, SocketAddr::new(Ipv4Addr::new(203, 0, 113, 10).into(), 19101));
    }

    #[test]
    fn turn_external_addr_falls_back_to_ipv6() {
        let addr = turn_external_addr(
            &PublicAddresses {
                ipv4: None,
                ipv6: Some(Ipv6Addr::LOCALHOST),
            },
            19101,
        )
        .unwrap();

        assert_eq!(addr, SocketAddr::new(Ipv6Addr::LOCALHOST.into(), 19101));
    }
}
