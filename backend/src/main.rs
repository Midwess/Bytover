use core_services::logger;
use devlog_sdk::api_gateway::client::ApiGatewayClient;
use devlog_sdk::api_gateway::kong::client::KongGatewayAdminClient;
use devlog_sdk::api_gateway::service::{GatewayRouteBuilder, GatewayRouteExpression, GatewayServiceBuilder};
use devlog_sdk::tcp::listener::{find_grpc_listener, find_tcp_listener, GrpcConnection, TcpConnection};
use schema::devlog::bitbridge::bit_bridge_cloud_service_server::BitBridgeCloudServiceServer;
use schema::devlog::bitbridge::p2p_orchestration_service_server::P2pOrchestrationServiceServer;
use tonic::transport::Server;
use tonic_middleware::InterceptorFor;

pub mod app_gateway;
pub mod cloud_storage;
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
    DiContainerError(#[from] di_container::DiContainerError)
}

#[tokio::main]
async fn main() -> Result<(), MainErrors> {
    logger::setup();
    let grpc_connection = find_grpc_listener(None).await?;
    let http_connection = find_tcp_listener(None).await?;

    let di = di_container::DiContainer::instance().await;
    di.start_cron_jobs().await?;

    setup_grpc_gateway(&grpc_connection).await?;
    setup_http_gateway(&http_connection).await?;

    let http_port = http_connection.port;
    let http_listener = http_connection.listener;

    // Spawn HTTP server - run() blocks but we handle ctrl+c in main
    let http_handle = tokio::spawn(async move {
        log::info!("Starting HTTP server on port {}", http_port);
        let std_listener = http_listener.into_std().expect("Failed to convert listener");
        actix_web::HttpServer::new(|| actix_web::App::new().configure(http::config))
            .listen(std_listener)
            .expect("Failed to bind HTTP server")
            .run()
            .await
    });

    // Wait for either gRPC server or Ctrl+C
    tokio::select! {
        result = start_grpc_server(grpc_connection) => {
            if let Err(e) = result {
                log::error!("gRPC server error: {}", e);
            }
        }
        result = tokio::signal::ctrl_c() => {
            match result {
                Ok(()) => {
                    log::info!("Received Ctrl+C, shutting down...");
                }
                Err(e) => {
                    log::error!("Failed to listen for Ctrl+C: {}", e);
                }
            }
        }
    }

    // Wait for HTTP server to stop (it will stop when runtime shuts down)
    log::info!("Waiting for HTTP server to stop...");
    if let Err(e) = http_handle.await {
        log::error!("HTTP server error: {}", e);
    }

    log::info!("Backend shutdown complete");

    Ok(())
}

async fn start_grpc_server(connection: GrpcConnection) -> Result<(), MainErrors> {
    let di = di_container::DiContainer::instance().await;
    log::info!("Start server at {}", connection.port);
    Server::builder()
        .add_service(InterceptorFor::new(
            BitBridgeCloudServiceServer::new(di.get_grpc_cloud_service().await),
            di.get_auth_middleware()
        ))
        .add_service(InterceptorFor::new(
            P2pOrchestrationServiceServer::new(di.get_grpc_p2p_service().await),
            di.get_auth_middleware()
        ))
        .serve_with_incoming(connection.listener)
        .await?;

    Ok(())
}

async fn setup_grpc_gateway(tcp: &GrpcConnection) -> Result<(), MainErrors> {
    log::info!("Registering with gateway");
    let api_gateway = KongGatewayAdminClient::new(devlog_sdk::config::CONFIGS.kong.admin_url.clone());

    let service = GatewayServiceBuilder::new()
        .grpc(tcp.public_host.clone(), tcp.port)
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

    log::info!("Register service {service:?}");
    api_gateway.register(service).await?;

    Ok(())
}

async fn setup_http_gateway(tcp: &TcpConnection) -> Result<(), MainErrors> {
    log::info!("Registering HTTP gateway");
    let api_gateway = KongGatewayAdminClient::new(devlog_sdk::config::CONFIGS.kong.admin_url.clone());

    let service = GatewayServiceBuilder::new()
        .http(tcp.public_host.clone(), tcp.port)
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

    log::info!("Register HTTP service {service:?}");
    api_gateway.register(service).await?;

    Ok(())
}
