use crate::app::operations::device::GeoLocation;
use crate::entities::finding_scope::FindingScope;
use crate::errors::NetworkError;
use core_services::retry;
use futures_util::lock::Mutex;
use n0_future::time::Instant;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone)]
pub struct InternetConnection {
    last_passed: Arc<Mutex<Option<Instant>>>,
    locator_server_url: String
}

#[derive(Debug, Deserialize, Serialize)]
pub struct NetworkResponse {
    pub ip_address: String,
    #[serde(default)]
    pub location_codes: Vec<String>
}

impl NetworkResponse {
    pub fn finding_scopes(&self) -> Vec<FindingScope> {
        let mut scopes = vec![FindingScope::Global(self.ip_address.clone())];

        for code in &self.location_codes {
            scopes.push(FindingScope::Local(code.to_string()))
        }

        scopes
    }
}

impl InternetConnection {
    pub fn new(locator_server_url: String) -> Self {
        Self {
            last_passed: Arc::new(Mutex::new(None)),
            locator_server_url
        }
    }

    pub async fn locate(&self, geo_location: Option<GeoLocation>) -> Result<NetworkResponse, NetworkError> {
        let client = reqwest::Client::new();

        let body = geo_location.map(|geo_location| serde_json::to_value(&geo_location).unwrap()).unwrap_or(json!({}));
        let response = retry!(retries = 30, delay = Duration::from_millis(250), |_| true, {
            let Ok(response) = client.post(&self.locator_server_url).json(&body).send().await else {
                return Err(NetworkError::Network("Failed to get public IP address".to_string()));
            };

            if response.status().is_success() {
                let network: NetworkResponse =
                    response.json().await.map_err(|_| NetworkError::Network("Bad response format".to_owned()))?;

                return Ok(network);
            }

            Err(NetworkError::Network("Failed to get public IP address".to_string()))
        })?;

        Ok(response)
    }

    pub async fn is_connected(&self) -> bool {
        let mut last_passed = self.last_passed.lock().await;
        if let Some(last_passed) = last_passed.as_ref() {
            if last_passed.elapsed() < Duration::from_secs(5) {
                return true;
            }
        }

        let ns = "internet-check";
        let addr = "https://devlog.studio/locator";
        let client = reqwest::Client::new();

        match client.post(addr).json(&json!({})).timeout(Duration::from_millis(5000)).send().await {
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
