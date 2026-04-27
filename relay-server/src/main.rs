mod config;
mod gateway;
mod geoip;
mod public_ip;

use base64::Engine;
use serde::Serialize;
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

use schema::devlog::bitbridge::p2p_orchestration_service_client::P2pOrchestrationServiceClient;
use std::env;
use tonic::Request;

use crate::gateway::GatewayChannel;
use crate::geoip::GeoipResolver;
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

    let turn_username = env::var("TURN_USERNAME").unwrap_or_else(|_| "relay".to_string());
    let turn_password = env::var("TURN_PASSWORD").unwrap_or_else(|_| "relay-secret".to_string());

    let turn_udp_send_buffer = env::var("TURN_UDP_SEND_BUFFER")
        .unwrap_or_else(|_| (2 * 1024 * 1024).to_string())
        .parse::<usize>()
        .unwrap();
    let turn_udp_recv_buffer = env::var("TURN_UDP_RECV_BUFFER")
        .unwrap_or_else(|_| (2 * 1024 * 1024).to_string())
        .parse::<usize>()
        .unwrap();

    let interfaces = build_turn_interfaces(
        &public_addresses,
        turn_port,
        turn_udp_send_buffer,
        turn_udp_recv_buffer,
    )
    .map_err(MainErrors::ExecutionError)?;

    let turn_config = Config {
        server: TurnServerConfig {
            interfaces,
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

    let region_resolver = RegionResolver::from_environment();
    log::info!("{}", region_resolver.startup_summary());

    let registration_state = RegistrationState::new(public_addresses, turn_username.clone(), turn_password.clone());

    let turn_handle = tokio::spawn(async move {
        if let Err(e) = turn_server::start_server(turn_config).await {
            log::error!("TURN server exited with error: {:?}", e);
        }
    });

    tokio::spawn(async move {
        start_registration_loop(turn_port, turn_port, turn_port, registration_state, region_resolver).await;
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

fn build_turn_interfaces(
    public_addresses: &PublicAddresses,
    port: u16,
    send_buffer_size: usize,
    recv_buffer_size: usize,
) -> Result<Vec<Interface>, String> {
    let mut interfaces = Vec::new();
    if let Some(ipv4) = public_addresses.ipv4 {
        interfaces.push(Interface::Udp {
            listen: format!("0.0.0.0:{}", port).parse().unwrap(),
            external: SocketAddr::new(ipv4.into(), port),
            idle_timeout: 20,
            mtu: 1500,
            demuxer_capacity: 4096,
            v6_only: false,
            send_buffer_size,
            recv_buffer_size,
            listener_count: 0,
        });
    }
    if let Some(ipv6) = public_addresses.ipv6 {
        interfaces.push(Interface::Udp {
            listen: format!("[::]:{}", port).parse().unwrap(),
            external: SocketAddr::new(ipv6.into(), port),
            idle_timeout: 20,
            mtu: 1500,
            demuxer_capacity: 4096,
            v6_only: true,
            send_buffer_size,
            recv_buffer_size,
            listener_count: 0,
        });
    }
    if interfaces.is_empty() {
        return Err("relay registration requires at least one public IP address".to_string());
    }
    Ok(interfaces)
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

struct RegionResolver {
    geoip: Option<GeoipResolver>,
    geoip_load_error: Option<String>,
}

impl RegionResolver {
    fn from_environment() -> Self {
        let path = config::geoip_db_path();
        match GeoipResolver::load(&path) {
            Ok(geoip) => Self { geoip: Some(geoip), geoip_load_error: None },
            Err(error) => Self { geoip: None, geoip_load_error: Some(error) },
        }
    }

    fn startup_summary(&self) -> String {
        match (&self.geoip, &self.geoip_load_error) {
            (Some(geoip), _) => format!("Region resolver initialized: env_override={:?} geoip_db={}", config::local_region_code(), geoip.db_path().display()),
            (None, Some(error)) => format!("Region resolver initialized without GeoIP: env_override={:?} reason={}", config::local_region_code(), error),
            (None, None) => format!("Region resolver initialized without GeoIP: env_override={:?}", config::local_region_code()),
        }
    }

    fn resolve_local(&self, addrs: &PublicAddresses) -> Option<(String, &'static str)> {
        let env_override = config::local_region_code();
        let geoip_region = self
            .geoip
            .as_ref()
            .and_then(|geoip| pick_geoip_region(addrs, |ip| geoip.region_for(ip)));
        resolve_local_region(env_override, geoip_region)
    }
}

fn pick_geoip_region<F>(addrs: &PublicAddresses, mut lookup: F) -> Option<&'static str>
where
    F: FnMut(IpAddr) -> Option<&'static str>,
{
    addrs
        .ipv6
        .and_then(|ip| lookup(IpAddr::V6(ip)))
        .or_else(|| addrs.ipv4.and_then(|ip| lookup(IpAddr::V4(ip))))
}

fn resolve_local_region(env_override: Option<String>, geoip_region: Option<&'static str>) -> Option<(String, &'static str)> {
    if let Some(env_value) = env_override {
        return Some((env_value, "env"));
    }
    geoip_region.map(|region| (region.to_string(), "geoip"))
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

async fn start_registration_loop(
    stun_port: u16,
    relay_port: u16,
    turn_port: u16,
    mut state: RegistrationState,
    region_resolver: RegionResolver,
) {
    let grpc_gateway = GatewayChannel::new(config::get_gateway_grpc_url(), config::get_gateway_grpc_host());

    let mut interval = tokio::time::interval(Duration::from_secs(5));
    loop {
        interval.tick().await;

        let url = match resolve_registration_url(&region_resolver, &grpc_gateway, &state.public_addresses).await {
            Ok((url, source)) => {
                if let Some((previous_url, next_url)) = state.update_registration_url(url.clone()) {
                    let ipv4_attribution = state.public_addresses.ipv4.map(|ip| ip.to_string()).unwrap_or_else(|| "none".to_string());
                    let ipv6_attribution = state.public_addresses.ipv6.map(|ip| ip.to_string()).unwrap_or_else(|| "none".to_string());
                    if let Some(previous_url) = previous_url {
                        log::info!(
                            "Relay registration URL changed from {} to {} (source={} ipv4={} ipv6={})",
                            previous_url, next_url, source, ipv4_attribution, ipv6_attribution
                        );
                    } else {
                        log::info!(
                            "Relay registration URL resolved to {} (source={} ipv4={} ipv6={})",
                            next_url, source, ipv4_attribution, ipv6_attribution
                        );
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

async fn resolve_registration_url(
    region_resolver: &RegionResolver,
    grpc_gateway: &GatewayChannel,
    addrs: &PublicAddresses,
) -> Result<(String, &'static str), String> {
    if let Some((region, source)) = region_resolver.resolve_local(addrs) {
        let route = format!("rpc-signalling-{region}");
        return Ok((config::get_signalling_registration_url(&route), source));
    }

    let channel = grpc_gateway.connect().await?;

    let mut client = P2pOrchestrationServiceClient::new(channel);
    let response = client
        .get_region(Request::new(schema::devlog::bitbridge::GetRegionRequest {}))
        .await
        .map_err(|error| format!("backend get_region failed: {error}"))?
        .into_inner();

    Ok((config::get_signalling_registration_url(&response.signalling_route), "grpc"))
}

#[cfg(test)]
mod tests {
    use super::{build_turn_interfaces, pick_geoip_region, resolve_local_region, RegistrationState};
    use crate::public_ip::PublicAddresses;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
    use turn_server::config::Interface;

    #[test]
    fn precedence_env_wins_over_geoip() {
        let resolved = resolve_local_region(Some("asia".to_string()), Some("us"));
        assert_eq!(resolved, Some(("asia".to_string(), "env")));
    }

    #[test]
    fn precedence_geoip_used_when_env_absent() {
        let resolved = resolve_local_region(None, Some("eu"));
        assert_eq!(resolved, Some(("eu".to_string(), "geoip")));
    }

    #[test]
    fn precedence_returns_none_when_both_absent() {
        let resolved = resolve_local_region(None, None);
        assert_eq!(resolved, None);
    }

    #[test]
    fn precedence_env_used_when_geoip_unmapped() {
        let resolved = resolve_local_region(Some("us".to_string()), None);
        assert_eq!(resolved, Some(("us".to_string(), "env")));
    }

    #[test]
    fn singapore_host_picks_v6_and_resolves_to_asia_not_us() {
        let addrs = PublicAddresses {
            ipv4: Some(Ipv4Addr::new(64, 118, 143, 14)),
            ipv6: Some("2404:c140:2100:6::46:6f69".parse().unwrap()),
        };
        let lookup = |ip: IpAddr| match ip {
            IpAddr::V4(v) if v == Ipv4Addr::new(64, 118, 143, 14) => Some("us"),
            IpAddr::V6(v) if v == "2404:c140:2100:6::46:6f69".parse::<Ipv6Addr>().unwrap() => {
                Some("asia")
            }
            _ => None,
        };
        assert_eq!(pick_geoip_region(&addrs, lookup), Some("asia"));
    }

    #[test]
    fn picker_falls_back_to_v4_when_v6_unmapped() {
        let addrs = PublicAddresses {
            ipv4: Some(Ipv4Addr::new(8, 8, 8, 8)),
            ipv6: Some("2c0f::1".parse().unwrap()),
        };
        let lookup = |ip: IpAddr| match ip {
            IpAddr::V4(v) if v == Ipv4Addr::new(8, 8, 8, 8) => Some("us"),
            _ => None,
        };
        assert_eq!(pick_geoip_region(&addrs, lookup), Some("us"));
    }

    #[test]
    fn picker_returns_none_when_neither_address_resolves() {
        let addrs = PublicAddresses {
            ipv4: Some(Ipv4Addr::new(203, 0, 113, 1)),
            ipv6: Some("2c0f::1".parse().unwrap()),
        };
        let lookup = |_: IpAddr| -> Option<&'static str> { None };
        assert_eq!(pick_geoip_region(&addrs, lookup), None);
    }

    #[test]
    fn picker_skips_v6_when_only_v4_present() {
        let addrs = PublicAddresses {
            ipv4: Some(Ipv4Addr::new(8, 8, 8, 8)),
            ipv6: None,
        };
        let lookup = |ip: IpAddr| match ip {
            IpAddr::V4(_) => Some("us"),
            IpAddr::V6(_) => panic!("v6 lookup must not be called when address is absent"),
        };
        assert_eq!(pick_geoip_region(&addrs, lookup), Some("us"));
    }

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

    fn udp_pair(iface: &Interface) -> (SocketAddr, SocketAddr, bool) {
        match iface {
            Interface::Udp { listen, external, v6_only, .. } => (*listen, *external, *v6_only),
            _ => panic!("expected Udp interface"),
        }
    }

    #[test]
    fn build_turn_interfaces_dual_family_creates_two_udp_interfaces() {
        let interfaces = build_turn_interfaces(
            &PublicAddresses {
                ipv4: Some(Ipv4Addr::new(203, 0, 113, 10)),
                ipv6: Some(Ipv6Addr::new(0x2404, 0xc140, 0x2100, 0, 0, 0, 0, 1)),
            },
            19101,
            8 * 1024 * 1024,
            8 * 1024 * 1024,
        )
        .unwrap();

        assert_eq!(interfaces.len(), 2);

        let (v4_listen, v4_external, v4_only) = udp_pair(&interfaces[0]);
        assert_eq!(v4_listen, "0.0.0.0:19101".parse::<SocketAddr>().unwrap());
        assert_eq!(v4_external, SocketAddr::new(Ipv4Addr::new(203, 0, 113, 10).into(), 19101));
        assert!(!v4_only);

        let (v6_listen, v6_external, v6_only) = udp_pair(&interfaces[1]);
        assert_eq!(v6_listen, "[::]:19101".parse::<SocketAddr>().unwrap());
        assert_eq!(v6_external, SocketAddr::new(Ipv6Addr::new(0x2404, 0xc140, 0x2100, 0, 0, 0, 0, 1).into(), 19101));
        assert!(v6_only);
    }

    #[test]
    fn build_turn_interfaces_v6_only_skips_v4_interface() {
        let interfaces = build_turn_interfaces(
            &PublicAddresses {
                ipv4: None,
                ipv6: Some(Ipv6Addr::LOCALHOST),
            },
            19101,
            8 * 1024 * 1024,
            8 * 1024 * 1024,
        )
        .unwrap();

        assert_eq!(interfaces.len(), 1);
        let (listen, external, v6_only) = udp_pair(&interfaces[0]);
        assert_eq!(listen, "[::]:19101".parse::<SocketAddr>().unwrap());
        assert_eq!(external, SocketAddr::new(Ipv6Addr::LOCALHOST.into(), 19101));
        assert!(v6_only);
    }

    #[test]
    fn build_turn_interfaces_no_addresses_errors() {
        let result = build_turn_interfaces(
            &PublicAddresses { ipv4: None, ipv6: None },
            19101,
            8 * 1024 * 1024,
            8 * 1024 * 1024,
        );
        assert!(result.is_err());
    }
}
