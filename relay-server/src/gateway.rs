use std::future::poll_fn;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Mutex;
use tonic::codegen::Service;
use tonic::transport::{Channel, ClientTlsConfig};

#[derive(Clone)]
pub struct GatewayChannel {
    channel: Arc<Mutex<Option<Channel>>>,
    endpoint: String,
    domain: String
}

impl GatewayChannel {
    pub fn new(endpoint: String, domain: String) -> Self {
        Self {
            channel: Arc::new(Mutex::new(None)),
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

    async fn new_connection(&self) -> Result<(), String> {
        let uri = self.endpoint.parse().map_err(|error| format!("invalid gateway endpoint: {error:?}"))?;
        let mut builder = Channel::builder(uri).connect_timeout(Duration::from_secs(5)).timeout(Duration::from_secs(12));

        if self.endpoint.starts_with("https://") {
            let tls = ClientTlsConfig::new().with_webpki_roots().domain_name(self.domain.clone());
            builder = builder.tls_config(tls).map_err(|error| format!("invalid gateway tls config: {error}"))?;
        }

        let channel = builder.connect().await.map_err(|error| format!("failed to connect to gateway: {error}"))?;

        self.channel.lock().await.replace(channel);

        Ok(())
    }

    pub async fn connect(&self) -> Result<Channel, String> {
        if self.is_connected().await {
            return Ok(self.channel.lock().await.clone().unwrap());
        }

        self.new_connection().await?;
        Ok(self.channel.lock().await.clone().unwrap())
    }
}
