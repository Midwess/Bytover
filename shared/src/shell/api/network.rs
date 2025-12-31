use crate::app::operations::device::GeoLocation;
use crate::entities::finding_scope::FindingScope;
use crate::errors::CoreError;
use core_services::retry;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Clone)]
pub struct InternetConnection {
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
        let mut scopes = vec![FindingScope::new(
            &self.ip_address
        )];

        for code in &self.location_codes {
            scopes.push(FindingScope::new(&format!("local://{}", code)))
        }

        scopes
    }
}

impl InternetConnection {
    pub fn new(locator_server_url: String) -> Self {
        Self { locator_server_url }
    }

    pub async fn locate(&self, geo_location: Option<GeoLocation>) -> Result<NetworkResponse, CoreError> {
        let client = reqwest::Client::new();

        let query = geo_location.and_then(|geo_location| serde_json::to_value(&geo_location).ok()).unwrap_or(json!({}));
        let response = retry!(retries = 3, delay = Duration::from_millis(3000), |_| true, {
            let Ok(response) = client.get(&self.locator_server_url).query(&query).send().await else {
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
