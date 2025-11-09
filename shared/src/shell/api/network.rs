use crate::app::operations::device::GeoLocation;
use crate::entities::finding_scope::FindingScope;
use crate::errors::CoreError;
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

    pub async fn locate(&self, geo_location: Option<GeoLocation>) -> Result<NetworkResponse, CoreError> {
        let client = reqwest::Client::new();

        let body = geo_location.map(|geo_location| serde_json::to_value(&geo_location).unwrap()).unwrap_or(json!({}));
        let response = retry!(retries = 3, delay = Duration::from_millis(3000), |_| true, {
            let Ok(response) = client.post(&self.locator_server_url).json(&body).send().await else {
                return Err(CoreError::Network("Failed to get public IP address".to_string()));
            };

            if response.status().is_success() {
                let network: NetworkResponse =
                    response.json().await.map_err(|_| CoreError::Network("Bad response format".to_owned()))?;

                return Ok(network);
            }

            Err(CoreError::Network("Failed to get public IP address".to_string()))
        })?;

        Ok(response)
    }
}
