use tokio::sync::OnceCell;

use crate::app_gateway::client::AppGatewayClient;
use crate::locator_client::LocatorClient;
use devlog_sdk::grpc_gateway::channel::GrpcGatewayChannel;

static DI_CONTAINER: OnceCell<DiContainer> = OnceCell::const_new();

pub struct DiContainer {
    pub public_ip: String,
    pub grpc_gateway_channel: GrpcGatewayChannel,
}

impl DiContainer {
    pub async fn init() -> &'static Self {
        DI_CONTAINER.get_or_init(|| async {
            let kong_host = devlog_sdk::config::CONFIGS.kong.host.clone();
            let kong_port = devlog_sdk::config::CONFIGS.kong.port;
            let public_host = devlog_sdk::config::CONFIGS.kong.host.clone();

            let locator_client = LocatorClient::new(kong_host, kong_port);

            let public_ip = match locator_client.get_public_ip().await {
                Ok(ip) => {
                    log::info!("Discovered public IP via locator: {}", ip);
                    ip
                }
                Err(e) => {
                    log::warn!("Failed to get public IP from locator, using public_host fallback: {}", e);
                    public_host
                }
            };

            let grpc_gateway_channel = GrpcGatewayChannel::new();

            DiContainer {
                public_ip,
                grpc_gateway_channel,
            }
        }).await
    }

    pub fn app_gateway_client(&self) -> AppGatewayClient {
        AppGatewayClient::new(self.grpc_gateway_channel.clone())
    }
}
