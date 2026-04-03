use tokio::sync::OnceCell;

use crate::grpc_middleware::auth::RelayAuthInterceptor;
use crate::locator_client::LocatorClient;

static DI_CONTAINER: OnceCell<DiContainer> = OnceCell::const_new();
use std::sync::Arc;
use crate::connection::proxy_manager::ProxyManager;

pub struct DiContainer {
    pub public_ip: String,
    pub proxy_manager: Arc<ProxyManager>,
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

            let proxy_manager = ProxyManager::new();
            proxy_manager.start();

            DiContainer {
                public_ip,
                proxy_manager,
            }
        }).await
    }

    pub fn get_auth_middleware(&self) -> RelayAuthInterceptor {
        RelayAuthInterceptor::new()
    }
}
