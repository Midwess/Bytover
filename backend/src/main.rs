use core_services::logger;
use devlog_sdk::api_gateway::client::ApiGatewayClient;
use devlog_sdk::api_gateway::kong::client::KongGatewayAdminClient;
use devlog_sdk::api_gateway::service::{GatewayRouteBuilder, GatewayRouteExpression, GatewayServiceBuilder};
use devlog_sdk::tcp::listener::{find_grpc_listener, GrpcConnection};
use schema::devlog::bitbridge::bit_bridge_cloud_service_server::BitBridgeCloudServiceServer;
use tonic::transport::Server;
use tonic_middleware::InterceptorFor;

pub mod app_gateway;
pub mod cloud_storage;
pub mod di_container;
pub mod entities;
pub mod errors;
pub mod grpc;
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
    TransportError(#[from] tonic::transport::Error)
}

#[tokio::main]
async fn main() -> Result<(), MainErrors> {
    logger::setup();
    let grpc_connection = find_grpc_listener().await?;

    setup_grpc_gateway(&grpc_connection).await?;
    start_grpc_server(grpc_connection).await?;

    Ok(())
}

async fn start_grpc_server(connection: GrpcConnection) -> Result<(), MainErrors> {
    let di = di_container::DiContainer::instance().await;
    log::info!("Start server at {}", connection.port);
    Server::builder()
        .accept_http1(true)
        .add_service(InterceptorFor::new(
            BitBridgeCloudServiceServer::new(di.get_grpc_cloud_service().await),
            di.get_auth_middleware()
        ))
        .serve_with_incoming(connection.listener)
        .await?;

    Ok(())
}

async fn setup_grpc_gateway(tcp: &GrpcConnection) -> Result<(), MainErrors> {
    log::info!("Registering with gateway");
    let api_gateway = KongGatewayAdminClient {
        url: devlog_sdk::config::CONFIGS.kong.admin_url.clone()
    };

    let service = GatewayServiceBuilder::new()
        .grpc(tcp.public_host.clone(), tcp.port)
        .name("bitbridge-grpc-server")
        .routes(vec![
            GatewayRouteBuilder::new()
                .grpc()
                .path(GatewayRouteExpression::proto_namespace("devlog.bitbridge"))
                .priority(i32::MAX)
                .strip_path(false)
                .public(true)
                .preserve_host(false)
                .grpc_web()
                .name("bitbridge-grpc-server-path")
                .build(),
        ])
        .build();

    log::info!("Register service {service:?}");
    api_gateway.register(service).await?;

    Ok(())
}
