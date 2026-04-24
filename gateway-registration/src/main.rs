use core_services::logger::setup;
use devlog_sdk::api_gateway::client::ApiGatewayClient;
use devlog_sdk::api_gateway::kong::client::KongGatewayAdminClient;
use devlog_sdk::api_gateway::service::{GatewayRouteBuilder, GatewayRouteExpression, GatewayServiceBuilder};
use log::{error, info};
use url::Url;

#[tokio::main]
async fn main() {
    setup();

    let kong_admin_url = std::env::var("KONG_ADMIN_URL").unwrap_or_else(|_| "http://kong-gateway:8001".to_string());

    let remote_gateway_url = std::env::var("REMOTE_GATEWAY_URL").unwrap_or_else(|_| "https://bytover.com".to_string());

    info!("Kong Admin URL: {}", kong_admin_url);
    info!("Remote Gateway URL: {}", remote_gateway_url);

    if let Err(e) = run(&kong_admin_url, &remote_gateway_url).await {
        error!("Failed to register gateway: {}", e);
        std::process::exit(1);
    }

    info!("Gateway registration completed successfully");
}

async fn run(kong_admin_url: &str, remote_gateway_url: &str) -> Result<(), Box<dyn std::error::Error>> {
    let parsed_url = Url::parse(remote_gateway_url)?;
    let host = parsed_url.host_str().ok_or("Invalid host")?;
    let scheme = parsed_url.scheme();

    let (http_url, grpc_url) = match scheme {
        "https" | "grpcs" => {
            let port = parsed_url.port().unwrap_or(443);
            (format!("https://{host}:{port}"), format!("grpcs://{host}:{port}"))
        }
        _ => {
            let port = parsed_url.port().unwrap_or(80);
            (format!("http://{host}:{port}"), format!("grpc://{host}:{port}"))
        }
    };

    let grpc_service = GatewayServiceBuilder::new()
        .name("app-gateway-grpc-server")
        .url(grpc_url)
        .enable_cors(true)
        .routes(vec![
            GatewayRouteBuilder::new()
                .grpc()
                .grpc_web()
                .path(GatewayRouteExpression::proto_namespace("devlog.app_gateway.rpc"))
                .priority(10)
                .strip_path(false)
                .public(true)
                .preserve_host(false)
                .name("app-gateway-grpc-server-path")
                .build(),
        ])
        .build();

    let http_service = GatewayServiceBuilder::new()
        .name("app-gateway-http-server")
        .url(http_url)
        .enable_cors(true)
        .routes(vec![
            GatewayRouteBuilder::new()
                .http()
                .path(GatewayRouteExpression::start_with("/app-gateway"))
                .priority(10)
                .strip_path(true)
                .public(true)
                .preserve_host(false)
                .name("app-gateway-http-server-path")
                .build(),
        ])
        .build();

    let client = KongGatewayAdminClient::new(kong_admin_url.to_string());

    wait_for_kong(&client).await?;

    client.register(grpc_service).await?;
    info!("Registered app-gateway-grpc-server");

    client.register(http_service).await?;
    info!("Registered app-gateway-http-server");

    Ok(())
}

async fn wait_for_kong(client: &KongGatewayAdminClient) -> Result<(), Box<dyn std::error::Error>> {
    let max_retries = 30;
    let retry_interval = 2;

    for i in 1..=max_retries {
        let kong_url = client.url();
        match reqwest::get(format!("{}/status", kong_url)).await {
            Ok(resp) if resp.status().is_success() => {
                info!("Kong Gateway is ready");
                return Ok(());
            }
            _ => {
                info!("Attempt {}/{} - Kong not ready, waiting {}s...", i, max_retries, retry_interval);
                tokio::time::sleep(tokio::time::Duration::from_secs(retry_interval)).await;
            }
        }
    }

    Err("Kong Gateway did not become ready in time".into())
}
