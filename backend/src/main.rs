use core_services::logger;
use devlog_sdk::api_gateway::client::ApiGatewayClient;
use devlog_sdk::api_gateway::kong::client::KongGatewayAdminClient;
use devlog_sdk::api_gateway::service::{GatewayRouteBuilder, GatewayRouteExpression, GatewayServiceBuilder};
use devlog_sdk::tcp::listener::find_tcp_listener;
use schema::devlog::bitbridge::bit_bridge_cloud_service_server::BitBridgeCloudServiceServer;
use schema::devlog::bitbridge::p2p_orchestration_service_server::P2pOrchestrationServiceServer;
use tonic::service::Routes;
use tonic_middleware::InterceptorFor;

pub mod app_gateway;
pub mod cloud_storage;
pub mod config;
pub mod di_container;
pub mod entities;
pub mod errors;
pub mod grpc;
pub mod http;
pub mod infrastructure;
pub mod mail;
pub mod repositories;
pub mod transfer;
pub mod user;

#[derive(thiserror::Error, Debug)]
enum MainErrors {
    #[error("Core service error {0}")]
    CoreServiceErrors(#[from] core_services::services::errors::Errors),
    #[error("Transport error {0}")]
    TransportError(#[from] tonic::transport::Error),
    #[error("DI container error {0}")]
    DiContainerError(#[from] di_container::DiContainerError),
    #[error("IO error {0}")]
    Io(#[from] std::io::Error),
}

#[tokio::main]
async fn main() -> Result<(), MainErrors> {
    logger::setup();

    let connection = find_tcp_listener(None).await?;
    let endpoint = config::resolve_public_endpoint(&connection.public_host, connection.port);

    let di = di_container::DiContainer::instance().await;
    di.start_cron_jobs().await?;

    setup_grpc_gateway(endpoint.host.clone(), endpoint.port).await?;
    setup_http_gateway(endpoint.host.clone(), endpoint.port).await?;

    let grpc_router = Routes::new(InterceptorFor::new(
        BitBridgeCloudServiceServer::new(di.get_grpc_cloud_service().await),
        di.get_auth_middleware(),
    ))
    .add_service(InterceptorFor::new(
        P2pOrchestrationServiceServer::new(di.get_grpc_p2p_service().await),
        di.get_auth_middleware(),
    ))
    .prepare()
    .into_axum_router();

    let app = grpc_router.merge(http::router());

    let local_addr = connection.listener.local_addr()?;
    log::info!("Starting unified HTTP/gRPC server on {}", local_addr);

    axum::serve(connection.listener, app)
        .with_graceful_shutdown(async {
            match tokio::signal::ctrl_c().await {
                Ok(()) => log::info!("Received Ctrl+C, shutting down..."),
                Err(e) => log::error!("Failed to listen for Ctrl+C: {}", e),
            }
        })
        .await?;

    log::info!("Backend shutdown complete");

    Ok(())
}

async fn setup_grpc_gateway(public_host: String, port: u16) -> Result<(), MainErrors> {
    log::info!("Registering with gateway");
    let api_gateway = KongGatewayAdminClient::new(devlog_sdk::config::CONFIGS.kong.admin_url.clone());

    let service = GatewayServiceBuilder::new()
        .grpc(public_host.clone(), port)
        .name("bitbridge-grpc-server")
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
                .name("bitbridge-grpc-server-path")
                .build(),
        ])
        .build();

    log::info!(
        "Register gRPC service {service:?} using upstream {}:{}",
        public_host,
        port
    );
    api_gateway.register(service).await?;

    Ok(())
}

async fn setup_http_gateway(public_host: String, port: u16) -> Result<(), MainErrors> {
    log::info!("Registering HTTP gateway");
    let api_gateway = KongGatewayAdminClient::new(devlog_sdk::config::CONFIGS.kong.admin_url.clone());

    let service = GatewayServiceBuilder::new()
        .http(public_host.clone(), port)
        .name("bitbridge-http-server")
        .enable_cors(true)
        .routes(vec![
            GatewayRouteBuilder::new()
                .http()
                .path(GatewayRouteExpression::start_with("/bitbridge/api/v1"))
                .priority(20)
                .strip_path(false)
                .public(true)
                .preserve_host(false)
                .name("bitbridge-http-server-path")
                .build(),
        ])
        .build();

    log::info!(
        "Register HTTP service {service:?} using upstream {}:{}",
        public_host,
        port
    );
    api_gateway.register(service).await?;

    Ok(())
}
