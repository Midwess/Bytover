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
    let port = parsed_url.port().unwrap_or(443);
    let scheme = parsed_url.scheme();

    let url = match scheme {
        "https" => format!("https://{}:{}", host, port),
        "grpcs" => format!("grpcs://{}:{}", host, port),
        "grpc" => format!("grpc://{}:{}", host, port),
        _ => format!("http://{}:{}", host, port)
    };

    let service = GatewayServiceBuilder::new()
        .name("app-gateway")
        .url(url)
        .enable_cors(true)
        .routes(vec![
            GatewayRouteBuilder::new()
                .name("app-gateway-proto".to_string())
                .grpc()
                .priority(10)
                .grpc_web()
                .path(GatewayRouteExpression::proto_namespace("devlog.app_gateway.rpc"))
                .preserve_host(false)
                .strip_path(false)
                .public(true)
                .build(),
        ])
        .build();

    let client = KongGatewayAdminClient::new(kong_admin_url.to_string());

    wait_for_kong(&client).await?;

    client.register(service).await?;

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
