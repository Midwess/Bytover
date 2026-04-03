use serde::Deserialize;

#[derive(Debug)]
pub enum LocatorError {
    HttpError(String),
    Timeout,
    ParseError,
}

impl std::fmt::Display for LocatorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LocatorError::HttpError(e) => write!(f, "HTTP error: {}", e),
            LocatorError::Timeout => write!(f, "Request timeout"),
            LocatorError::ParseError => write!(f, "Failed to parse response"),
        }
    }
}

#[derive(Deserialize)]
struct LocateResponse {
    ip_address: String,
}

pub struct LocatorClient {
    kong_host: String,
    kong_port: u16,
}

impl LocatorClient {
    pub fn new(kong_host: String, kong_port: u16) -> Self {
        Self { kong_host, kong_port }
    }

    pub async fn get_public_ip(&self) -> Result<String, LocatorError> {
        let url = format!("http://{}:{}/locator", self.kong_host, self.kong_port);

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(2))
            .build()
            .map_err(|e| LocatorError::HttpError(e.to_string()))?;

        let response = client
            .get(&url)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    LocatorError::Timeout
                } else {
                    LocatorError::HttpError(e.to_string())
                }
            })?;

        let response: LocateResponse = response
            .json()
            .await
            .map_err(|_| LocatorError::ParseError)?;

        Ok(response.ip_address)
    }
}
