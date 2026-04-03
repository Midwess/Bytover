mod connection;
mod di;
mod grpc_service;
mod grpc_middleware;
mod locator_client;

use devlog_sdk::api_gateway::client::ApiGatewayClient;
use devlog_sdk::api_gateway::kong::client::KongGatewayAdminClient;
use devlog_sdk::api_gateway::service::{GatewayRouteBuilder, GatewayRouteExpression, GatewayServiceBuilder};
use devlog_sdk::tcp::listener::{find_grpc_listener, GrpcConnection};
use tonic::transport::Server;
use tonic_middleware::InterceptorFor;
use schema::devlog::bitbridge::relay_service_server::RelayServiceServer;

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

    log::info!("Starting relay server...");

    let di = DiContainer::init().await;
    let public_ip = di.public_ip.clone();

    let connection = find_grpc_listener(Some(9101)).await.map_err(|e| MainErrors::DiContainerError(e.to_string()))?;
    let port = connection.port;

    setup_grpc_gateway(&connection).await?;

    log::info!("Relay Server listening on {}:{}", public_ip, port);

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
