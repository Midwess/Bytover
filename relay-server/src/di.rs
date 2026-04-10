use std::net::Ipv4Addr;
use std::sync::Arc;
use tokio::sync::OnceCell;

use crate::connection::proxy_manager::ProxyManager;
use crate::grpc_middleware::auth::RelayAuthInterceptor;

static DI_CONTAINER: OnceCell<DiContainer> = OnceCell::const_new();

pub struct DiContainer {
    pub proxy_manager: Arc<ProxyManager>
}

impl DiContainer {
    pub async fn init(public_ip: Ipv4Addr) -> &'static Self {
        DI_CONTAINER
            .get_or_init(|| async {
                let proxy_manager = ProxyManager::new(public_ip);
                {
                    let pm = proxy_manager.clone();
                    tokio::spawn(async move {
                        pm.start().await;
                    });
                }

                DiContainer { proxy_manager }
            })
            .await
    }

    pub fn get_auth_middleware(&self) -> RelayAuthInterceptor {
        RelayAuthInterceptor::new()
    }
}
