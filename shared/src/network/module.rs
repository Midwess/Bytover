use std::time::{Duration, Instant};

use tokio::sync::Mutex;

use crate::errors::NetworkError;

#[async_trait::async_trait]
pub trait NetworkModule {
    // Check if the module is connected to the upstream
    async fn is_connected(&self) -> bool;
    // The module could try to reconnect it self, we need to wait until it is connected
    async fn wait_until_connected(&self, timeout: Duration) {
        let elapsed = Instant::now();
        while elapsed.elapsed() < timeout {
            if self.is_connected().await {
                break;
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
    // Call this method will cause module to reconnect to the upstream
    // Even if it is already connected
    async fn connect(&self, timeout: Duration) -> Result<(), NetworkError>;
}

pub struct InternetConnection {
    last_passed: Mutex<Instant>
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

    pub async fn is_connected(&self) -> bool {
        let mut last_passed = self.last_passed.lock().await;
        if last_passed.elapsed() < Duration::from_secs(5) {
            return true;
        }

        let ns = "internet-check";
        // This endpoint is located in Digitalocean
        let addr =
            "https://faas-sgp1-18bc02ac.doserverless.co/api/v1/web/fn-40c6321e-1ea6-4748-bfec-44cee2c996d5/default/network-check";
        let client = reqwest::Client::new();

        // Timeout is 5 seconds seem too much, but it is neccessary for cross region connection
        // And for Digital ocean sometime has a cold start which take more time than usual
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
