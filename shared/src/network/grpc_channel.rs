use std::future::poll_fn;
use std::pin::Pin;
use std::time::Duration;

use tokio::sync::Mutex;
use tonic::client::GrpcService;
use tonic::transport::{Channel, ClientTlsConfig, Endpoint};

use crate::errors::NetworkError;

use super::module::{InternetConnection, NetworkModule};

pub struct GrpcChannel {
    channel: Mutex<Option<Channel>>,
    endpoint: Endpoint,
    internet_connection: InternetConnection
}

impl GrpcChannel {
    pub fn new(endpoint: Endpoint) -> Self {
        Self {
            channel: Mutex::new(None),
            endpoint,
            internet_connection: InternetConnection::new()
        }
    }

    pub async fn reconnect(&self) -> Result<Channel, NetworkError> {
        NetworkModule::connect(self, Duration::from_secs(5)).await?;
        Ok(self.channel.lock().await.clone().unwrap())
    }

    pub async fn connect(&self) -> Result<Channel, NetworkError> {
        if self.is_connected().await {
            return Ok(self.channel.lock().await.clone().unwrap())
        }

        if !self.internet_connection.is_connected().await {
            return Err(NetworkError::Network("No internet".to_string()));
        }

        NetworkModule::connect(self, Duration::from_secs(5)).await?;
        Ok(self.channel.lock().await.clone().unwrap())
    }
}

#[async_trait::async_trait]
impl NetworkModule for GrpcChannel {
    async fn is_connected(&self) -> bool {
        if let Some(mut channel) = self.channel.lock().await.clone() {
            return poll_fn(|cx| Pin::new(&mut channel).poll_ready(cx)).await.is_ok()
        }

        false
    }

    async fn connect(&self, timeout: Duration) -> Result<(), NetworkError> {
        let tls = ClientTlsConfig::new()
            .domain_name("grpc.devlog.studio");

        let mut channel = self.endpoint.clone()
            .connect_timeout(timeout)
            .timeout(Duration::from_secs(10))
            .connect()
            .await?;

        poll_fn(|cx| Pin::new(&mut channel).poll_ready(cx)).await?;

        self.channel.lock().await.replace(channel);

        Ok(())
    }
}
