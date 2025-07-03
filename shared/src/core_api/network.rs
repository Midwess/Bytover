use std::sync::Arc;
use serde::{Deserialize, Serialize};
use crate::errors::NetworkError;
use std::time::{Duration, Instant};
use futures_timer::Delay;
use futures_util::lock::Mutex;

#[derive(Clone)]
pub struct InternetConnection {
    last_passed: Arc<Mutex<Instant>>
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
            last_passed: Arc::new(Mutex::new(Instant::now() - Duration::from_secs(5)))
        }
    }

    pub async fn ip_address(&self) -> Result<String, NetworkError> {
        const MAX_RETRIES: usize = 30;
        const RETRY_DELAY_MS: u64 = 500;

        let client = reqwest::Client::new();

        for _ in 0..MAX_RETRIES {
            if let Ok(response) = client.get("https://network-info.devlog.studio").send().await {
                if response.status().is_success() {
                    let network: NetworkResponse =
                        response.json().await.map_err(|_| NetworkError::Network("Bad response format".to_owned()))?;

                    return Ok(network.ip);
                }
            }

            Delay::new(Duration::from_millis(RETRY_DELAY_MS)).await;
        }

        Err(NetworkError::Network("Failed to get public IP address".to_string()))
    }

    pub async fn is_connected(&self) -> bool {
        let mut last_passed = self.last_passed.lock().await;
        if last_passed.elapsed() < Duration::from_secs(5) {
            return true;
        }

        let ns = "internet-check";
        let addr = "https://network-info.devlog.studio";
        let client = reqwest::Client::new();

        match client.get(addr).timeout(Duration::from_millis(5000)).send().await {
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
