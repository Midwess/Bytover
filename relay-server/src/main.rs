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
use devlog_sdk::tcp::listener::find_grpc_listener;
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
    #[error("Execution error {0}")]
    ExecutionError(String),
}

#[tokio::main]
async fn main() -> Result<(), MainErrors> {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    log::info!("Starting relay server...");

    let di = DiContainer::init().await;
    let public_ip = di.public_ip.clone();

    // Prepare STUN server
    let stun_port = 3478;
    let udp_conn = find_udp_socket(Some(stun_port))
        .await
        .map_err(|e| MainErrors::DiContainerError(e.to_string()))?;

    // Prepare gRPC server
    let connection = find_grpc_listener(Some(9101))
        .await
        .map_err(|e| MainErrors::DiContainerError(e.to_string()))?;
    let relay_port = connection.port;

    log::info!("Relay Server listening on {}:{}", public_ip, relay_port);

    // Start registration loop
    tokio::spawn(async move {
        start_registration_loop(stun_port, relay_port).await;
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
}

async fn start_registration_loop(stun_port: u16, relay_port: u16) {
    let signalling_url = env::var("SIGNALLING_URL").unwrap_or_else(|_| "http://localhost:9221".to_string());
    
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
