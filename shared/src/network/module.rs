use futures_util::TryFutureExt;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

use crate::errors::NetworkError;

#[async_trait::async_trait]
pub trait NetworkModule {
    // Check if the module is connected to the upstream
    async fn is_connected(&self) -> bool;
    // The module could try to reconnect it's self, we need to wait until it is connected
    async fn wait_until_connected(&self, timeout: Duration) {
        let elapsed = Instant::now();
        while elapsed.elapsed() < timeout {
            if self.is_connected().await {
                break;
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
    // Call this method will cause this module to reconnect to the upstream
    // Even if it is already connected
    async fn connect(&self, timeout: Duration) -> Result<(), NetworkError>;
}

pub struct InternetConnection {
    last_passed: Mutex<Instant>
}

#[derive(Debug, Deserialize, Serialize)]
pub struct NetworkResponse {
    ip: String
}

impl Default for InternetConnection {
    fn default() -> Self {
        Self::new()
    }
}

impl InternetConnection {
    pub fn new() -> Self {
        Self {
            last_passed: Mutex::new(Instant::now() - Duration::from_secs(5))
        }
    }

    pub async fn ip_address(&self) -> Result<String, NetworkError> {
        const MAX_RETRIES: usize = 30;
        const RETRY_DELAY_MS: u64 = 500;

        let client = reqwest::Client::new();

        for _ in 0..MAX_RETRIES {
            if let Ok(response) = client.get("https://network-info.up.railway.app").send().await {
                if response.status().is_success() {
                    let network: NetworkResponse =
                        response.json().await.map_err(|it| NetworkError::Network("Bad response format".to_owned()))?;

                    return Ok(network.ip);
                }
            }

            tokio::time::sleep(Duration::from_millis(RETRY_DELAY_MS)).await;
        }

        Err(NetworkError::Network("Failed to get public IP address".to_string()))
    }

    pub async fn is_connected(&self) -> bool {
        let mut last_passed = self.last_passed.lock().await;
        if last_passed.elapsed() < Duration::from_secs(5) {
            return true;
        }

        let ns = "internet-check";
        let addr = "https://network-info.up.railway.app";
        let client = reqwest::Client::new();

        match client.get(addr).timeout(Duration::from_millis(3000)).send().await {
            Ok(_) => {
                *last_passed = Instant::now();
                true
            }
            Err(err) => {
                log::info!(
                    target: ns,
                    "No internet connection in the last {} seconds: {:?}",
                    last_passed.elapsed().as_secs(),
                    err
                );
                false
            }
        }
    }
}
