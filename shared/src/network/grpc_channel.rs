use std::time::Duration;

use tokio::sync::Mutex;
use tonic::transport::{Channel, Endpoint};

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
            internet_connection: InternetConnection::new(),
        }
    }

    pub async fn connect(&self) -> Result<Channel, NetworkError> {
        NetworkModule::connect(self, Duration::from_secs(5)).await?;
        Ok(self.channel.lock().await.clone().unwrap())
    }
}

#[async_trait::async_trait]
impl NetworkModule for GrpcChannel {
    async fn is_connected(&self) -> bool {
        if !self.internet_connection.is_connected().await {
            return false;
        }

        self.channel.lock().await.is_some()
    }

    async fn connect(&self, timeout: Duration) -> Result<(), NetworkError> {
        if self.is_connected().await {
            return Ok(());
        }

        if !self.internet_connection.is_connected().await {
            return Err(NetworkError::Network("No internet".to_string()));
        }

        let channel = self.endpoint.clone().connect_timeout(timeout).connect().await?;

        self.channel.lock().await.replace(channel);

        Ok(())
    }
}
