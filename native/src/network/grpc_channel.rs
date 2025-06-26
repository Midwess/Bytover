use std::future::poll_fn;
use std::time::Duration;

use super::module::{InternetConnection, NetworkModule};
use crate::grpc::errors::NativeGrpcErrors;
use shared::errors::NetworkError;
use tokio::sync::Mutex;
use tonic::client::GrpcService;
use tonic::transport::{Channel, ClientTlsConfig};

pub struct GrpcClient {
    channel: Mutex<Option<Channel>>,
    internet_connection: InternetConnection,
    endpoint: String
}

impl GrpcClient {
    pub fn new(endpoint: String) -> Self {
        Self {
            channel: Mutex::new(None),
            endpoint,
            internet_connection: InternetConnection::new()
        }
    }

    pub async fn reconnect(&self) -> Result<Channel, NativeGrpcErrors> {
        NetworkModule::connect(self, Duration::from_secs(5)).await?;
        Ok(self.channel.lock().await.clone().unwrap())
    }

    pub async fn connect(&self) -> Result<Channel, NativeGrpcErrors> {
        if self.is_connected().await {
            return Ok(self.channel.lock().await.clone().unwrap())
        }

        if !self.internet_connection.is_connected().await {
            return Err(NativeGrpcErrors::Connection(NetworkError::Network("No internet".to_string())));
        }

        NetworkModule::connect(self, Duration::from_secs(5)).await?;
        Ok(self.channel.lock().await.clone().unwrap())
    }
}

#[async_trait::async_trait]
impl NetworkModule for GrpcClient {
    async fn is_connected(&self) -> bool {
        if let Some(mut channel) = self.channel.lock().await.clone() {
            return poll_fn(|cx| channel.poll_ready(cx)).await.is_ok();
        }

        false
    }

    async fn connect(&self, timeout: Duration) -> Result<(), NetworkError> {
        let mut builder = Channel::builder(self.endpoint.parse().unwrap())
            .connect_timeout(timeout)
            .timeout(Duration::from_millis(12000));

        if self.endpoint.starts_with("https") {
            let tls = ClientTlsConfig::new().with_webpki_roots();

            builder = builder.tls_config(tls).map_err(NativeGrpcErrors::from)?;
        };

        let channel = builder.connect().await.map_err(NativeGrpcErrors::from)?;
        self.channel.lock().await.replace(channel);

        Ok(())
    }
}
