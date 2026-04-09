use shared::protocol::rpc::connection::RpcNetworkModule;
use shared::protocol::rpc::errors::RpcErrors;
use std::future::poll_fn;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tonic::codegen::Service;
use tonic::transport::{Channel, ClientTlsConfig};

#[derive(Clone)]
pub struct RpcNetworkModuleImpl {
    channel: Arc<Mutex<Option<Channel>>>,
    endpoint: String,
    domain: String
}

impl RpcNetworkModuleImpl {
    pub fn new(endpoint: String, domain: String) -> Self {
        Self {
            channel: Default::default(),
            endpoint,
            domain
        }
    }

    async fn is_connected(&self) -> bool {
        if let Some(mut channel) = self.channel.lock().await.clone() {
            return poll_fn(|cx| channel.poll_ready(cx)).await.is_ok();
        }

        false
    }

    pub async fn new_connection(&self) -> Result<(), RpcErrors> {
        let uri = self.endpoint.parse().map_err(|e| RpcErrors::InternalServerError(format!("{e:?}")))?;
        let mut builder = Channel::builder(uri).connect_timeout(Duration::from_secs(5)).timeout(Duration::from_millis(12000));

        if self.endpoint.starts_with("https") {
            log::info!("Connecting to gRPC server over TLS");
            let tls = ClientTlsConfig::new().with_webpki_roots().domain_name(self.domain.clone());
            builder = builder.tls_config(tls).unwrap();
        };

        let channel = builder.connect().await.map_err(|e| RpcErrors::InternalServerError(format!("{e:?}")))?;
        self.channel.lock().await.replace(channel);

        Ok(())
    }
}

#[async_trait::async_trait]
impl RpcNetworkModule<Channel> for RpcNetworkModuleImpl {
    async fn connect(&self) -> Result<Channel, RpcErrors> {
        if self.is_connected().await {
            return Ok(self.channel.lock().await.clone().unwrap())
        }

        self.new_connection().await?;
        Ok(self.channel.lock().await.clone().unwrap())
    }
}
