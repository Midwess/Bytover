use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Mutex;
use tonic::transport::{Channel, Endpoint};

use crate::config::CONFIGS;

// A singleton channel that open a connection to the gateway
// and because every services is connected to gateway, they all be shared the same channel
#[derive(Clone)]
pub struct GrpcGatewayChannel {
    channel: Arc<Mutex<Option<Channel>>>,
    endpoint: Endpoint
}

impl Default for GrpcGatewayChannel {
    fn default() -> Self {
        Self::new()
    }
}

impl GrpcGatewayChannel {
    pub fn new() -> Self {
        Self {
            channel: Arc::new(Mutex::new(None)),
            endpoint: format!("grpc://{}:{}", CONFIGS.kong.host, CONFIGS.kong.port).parse().unwrap()
        }
    }

    pub async fn connect(&self) -> Result<Channel, tonic::transport::Error> {
        self.connect_with_timeout(Duration::from_secs(5)).await?;
        Ok(self.channel.lock().await.clone().unwrap())
    }

    pub async fn is_connected(&self) -> bool {
        self.channel.lock().await.is_some()
    }

    pub async fn connect_with_timeout(&self, timeout: Duration) -> Result<(), tonic::transport::Error> {
        if self.is_connected().await {
            return Ok(());
        }

        let channel = self.endpoint.clone().connect_timeout(timeout).connect().await?;

        self.channel.lock().await.replace(channel);

        Ok(())
    }
}
