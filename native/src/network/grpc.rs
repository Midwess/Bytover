use crate::config::get_locator_url;
use shared::protocol::rpc::connection::RpcNetworkModule;
use shared::protocol::rpc::errors::RpcErrors;
use shared::shell::api::network::InternetConnection;
use std::future::poll_fn;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tonic::codegen::Service;
use tonic::transport::{Channel, ClientTlsConfig};

#[derive(Clone)]
pub struct RpcNetworkModuleImpl {
    channel: Arc<Mutex<Option<Channel>>>,
    internet_connection: InternetConnection,
    endpoint: String
}

impl RpcNetworkModuleImpl {
    pub fn new(endpoint: String) -> Self {
        Self {
            channel: Default::default(),
            endpoint,
            internet_connection: InternetConnection::new(get_locator_url())
        }
    }

    async fn is_connected(&self) -> bool {
        if let Some(mut channel) = self.channel.lock().await.clone() {
            return poll_fn(|cx| channel.poll_ready(cx)).await.is_ok();
        }

        false
    }

    pub async fn new_connection(&self) -> Result<(), RpcErrors> {
        let mut builder = Channel::builder(self.endpoint.parse().unwrap())
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_millis(12000));

        if self.endpoint.starts_with("https") {
            let tls = ClientTlsConfig::new().with_native_roots();

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

        if !self.internet_connection.is_connected().await {
            return Err(RpcErrors::InternalServerError("No internet".to_string()));
        }

        self.new_connection().await?;
        Ok(self.channel.lock().await.clone().unwrap())
    }
}
