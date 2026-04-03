mod connection;
mod di;
mod grpc_service;
mod grpc_middleware;
mod locator_client;
mod stun_server;

use std::time::Duration;
use serde::Serialize;
use base64::Engine;

use std::env;
use std::process::Command;

use devlog_sdk::api_gateway::client::ApiGatewayClient;
use devlog_sdk::api_gateway::kong::client::KongGatewayAdminClient;
use devlog_sdk::api_gateway::service::{GatewayRouteBuilder, GatewayRouteExpression, GatewayServiceBuilder};
use devlog_sdk::tcp::listener::{find_grpc_listener, GrpcConnection};
use tonic::transport::Server;
use tonic_middleware::InterceptorFor;
use schema::devlog::bitbridge::relay_service_server::RelayServiceServer;
use devlog_sdk::tcp::listener::find_udp_socket;

use crate::di::DiContainer;
use crate::grpc_service::RelayServiceImpl;

#[derive(thiserror::Error, Debug)]
enum MainErrors {
    #[error("Transport error {0}")]
    TransportError(#[from] tonic::transport::Error),
    #[error("DI container error {0}")]
    DiContainerError(String),
    #[error("Gateway error {0}")]
    GatewayError(String),
}

#[tokio::main]
async fn main() -> Result<(), MainErrors> {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    let args: Vec<String> = env::args().collect();
    if args.iter().any(|arg| arg == "--stun") {
        log::info!("Running as STUN server...");
        let udp_conn = find_udp_socket(Some(3478)).await.map_err(|e| MainErrors::DiContainerError(e.to_string()))?;
        stun_server::run_stun_server(udp_conn).await.map_err(|e| MainErrors::DiContainerError(e.to_string()))?;
        return Ok(());
    }

    log::info!("Starting relay server...");

    // Start STUN server process
    let exe = env::current_exe().map_err(|e| MainErrors::DiContainerError(e.to_string()))?;
    log::info!("Spawning STUN server process: {:?}", exe);
    let mut stun_child = Command::new(exe)
        .arg("--stun")
        .spawn()
        .map_err(|e| MainErrors::DiContainerError(e.to_string()))?;

    tokio::spawn(async move {
        let status = stun_child.wait();
        log::error!("STUN server process exited with status: {:?}", status);
    });

    let di = DiContainer::init().await;
    let public_ip = di.public_ip.clone();

    let connection = find_grpc_listener(Some(9101)).await.map_err(|e| MainErrors::DiContainerError(e.to_string()))?;
    let port = connection.port;

    setup_grpc_gateway(&connection).await?;

    log::info!("Relay Server listening on {}:{}", public_ip, port);

    // Start registration loop
    let stun_port = 3478; // Default, will be used by STUN child
    let relay_port = port;
    tokio::spawn(async move {
        start_registration_loop(stun_port, relay_port).await;
    });

    Server::builder()
        .add_service(InterceptorFor::new(
            RelayServiceServer::new(RelayServiceImpl::new(di.proxy_manager.clone())),
            di.get_auth_middleware()
        ))
        .serve_with_incoming(connection.listener)
        .await?;

    Ok(())
}

async fn setup_grpc_gateway(tcp: &GrpcConnection) -> Result<(), MainErrors> {
    log::info!("Registering relay with gateway");
    let api_gateway = KongGatewayAdminClient::new(devlog_sdk::config::CONFIGS.kong.admin_url.clone());

    let service = GatewayServiceBuilder::new()
        .grpc(tcp.public_host.clone(), tcp.port)
        .name("bitbridge-relay-server")
        .enable_cors(true)
        .routes(vec![
            GatewayRouteBuilder::new()
                .grpc()
                .grpc_web()
                .path(GatewayRouteExpression::proto_namespace("devlog.bitbridge"))
                .priority(10)
                .strip_path(false)
                .public(true)
                .preserve_host(false)
                .name("bitbridge-relay-server-path")
                .build(),
        ])
        .build();

    log::info!("Register relay service {service:?}");
    api_gateway.register(service).await.map_err(|e| MainErrors::GatewayError(e.to_string()))?;

    Ok(())
}

#[derive(Serialize)]
struct RegisterRelayRequest {
    stun_port: u16,
    relay_port: u16,
}

async fn start_registration_loop(stun_port: u16, relay_port: u16) {
    let signalling_url = env::var("SIGNALLING_URL").unwrap_or_else(|_| "http://localhost:9102".to_string());
    
    let secret = env::var("RELAY_SERVER_SECRET").unwrap_or_else(|_| "supersecret".to_string());
    let auth_str = format!("user:{}", secret);
    let b64_auth = base64::engine::general_purpose::STANDARD.encode(auth_str);
    let auth_header = format!("Basic {}", b64_auth);

    let client = reqwest::Client::new();
    let url = format!("{}/register-relay", signalling_url.trim_end_matches('/'));

    let mut interval = tokio::time::interval(Duration::from_secs(5));
    loop {
        interval.tick().await;

        let req_body = RegisterRelayRequest {
            stun_port,
            relay_port,
        };

        match client.post(&url)
            .header("authorization", &auth_header)
            .json(&req_body)
            .send()
            .await 
        {
            Ok(resp) => {
                if !resp.status().is_success() {
                    log::error!("Failed to register relay: status {}", resp.status());
                }
            },
            Err(e) => {
                log::error!("Error sending registration request: {}", e);
            }
        }
    }
}
