use crate::errors::NetworkError;
use futures_timer::Delay;
use futures_util::lock::Mutex;
use n0_future::time::Instant;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone)]
pub struct InternetConnection {
    last_passed: Arc<Mutex<Option<Instant>>>
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
            last_passed: Arc::new(Mutex::new(None))
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
        if let Some(last_passed) = last_passed.as_ref() {
            if last_passed.elapsed() < Duration::from_secs(5) {
                return true;
            }
        }

        let ns = "internet-check";
        let addr = "https://network-info.devlog.studio";
        let client = reqwest::Client::new();

        match client.get(addr).timeout(Duration::from_millis(5000)).send().await {
            Ok(_) => {
                *last_passed = Some(Instant::now());
                true
            }
            Err(err) => {
                log::info!(
                    target: ns,
                    "No internet connection in the last {:?}: {:?}",
                    last_passed.as_ref(),
                    err
                );
                false
            }
        }
    }
}
