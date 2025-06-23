use std::time::Duration;

use tokio::sync::Mutex;
use tonic_web_wasm_client::Client;

use crate::errors::NetworkError;

use super::module::{InternetConnection, NetworkModule};

pub struct GrpcClient {
    client: Mutex<Option<Client>>,
    internet_connection: InternetConnection,
    endpoint: String
}

impl GrpcClient {
    pub fn new(endpoint: String) -> Self {
        Self {
            client: Mutex::new(None),
            endpoint,
            internet_connection: InternetConnection::new()
        }
    }

    pub async fn reconnect(&self) -> Result<Client, NetworkError> {
        NetworkModule::connect(self, Duration::from_secs(5)).await?;
        Ok(self.client.lock().await.clone().unwrap())
    }

    pub async fn connect(&self) -> Result<Client, NetworkError> {
        if self.is_connected().await {
            return Ok(self.client.lock().await.clone().unwrap())
        }

        if !self.internet_connection.is_connected().await {
            return Err(NetworkError::Network("No internet".to_string()));
        }

        NetworkModule::connect(self, Duration::from_secs(5)).await?;
        Ok(self.client.lock().await.clone().unwrap())
    }
}

#[async_trait::async_trait]
impl NetworkModule for GrpcClient {
    async fn is_connected(&self) -> bool {
        if !self.internet_connection.is_connected().await {
            return false;
        }

        self.client.lock().await.is_some()
    }

    async fn connect(&self, timeout: Duration) -> Result<(), NetworkError> {
        let client = Client::new(self.endpoint.clone());

        self.client.lock().await.replace(client);

        Ok(())
    }
}
