use crate::config::AppStoreConnectApiCredentials;
use async_trait::async_trait;
use chrono::Utc;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

const JWT_AUDIENCE: &str = "appstoreconnect-v1";
const JWT_LIFETIME: Duration = Duration::from_secs(20 * 60);
const HTTP_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone)]
pub struct AppStoreVersionInfo {
    pub version_string: String,
    pub apple_platform: String,
}

#[derive(Debug, thiserror::Error)]
pub enum AscApiError {
    #[error("JWT signing failed: {0}")]
    JwtSigning(#[from] jsonwebtoken::errors::Error),
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("App Store Connect API returned status {0}")]
    Status(reqwest::StatusCode),
    #[error("response missing field '{0}'")]
    MissingField(&'static str),
}

#[async_trait]
pub trait AppStoreConnectApi: Send + Sync {
    async fn fetch_app_store_version(&self, id: &str) -> Result<AppStoreVersionInfo, AscApiError>;
}

pub struct ReqwestAppStoreConnectApi {
    credentials: AppStoreConnectApiCredentials,
    base_url: String,
    http: Client,
}

impl ReqwestAppStoreConnectApi {
    pub fn new(credentials: AppStoreConnectApiCredentials, base_url: String) -> Self {
        let http = Client::builder()
            .timeout(HTTP_TIMEOUT)
            .build()
            .unwrap_or_else(|err| panic!("Failed to build reqwest client: {err}"));
        Self {
            credentials,
            base_url,
            http,
        }
    }

    fn sign_jwt(&self) -> Result<String, AscApiError> {
        let now = Utc::now().timestamp();
        let exp = now + JWT_LIFETIME.as_secs() as i64;

        #[derive(Serialize)]
        struct Claims<'a> {
            iss: &'a str,
            iat: i64,
            exp: i64,
            aud: &'a str,
        }

        let claims = Claims {
            iss: &self.credentials.issuer_id,
            iat: now,
            exp,
            aud: JWT_AUDIENCE,
        };

        let mut header = Header::new(Algorithm::ES256);
        header.kid = Some(self.credentials.key_id.clone());
        header.typ = Some("JWT".to_string());

        let key = EncodingKey::from_ec_pem(self.credentials.private_key_pem.as_bytes())?;
        Ok(encode(&header, &claims, &key)?)
    }
}

#[async_trait]
impl AppStoreConnectApi for ReqwestAppStoreConnectApi {
    async fn fetch_app_store_version(&self, id: &str) -> Result<AppStoreVersionInfo, AscApiError> {
        let token = self.sign_jwt()?;
        let url = format!("{}/v1/appStoreVersions/{}", self.base_url.trim_end_matches('/'), id);

        let response = self
            .http
            .get(&url)
            .bearer_auth(token)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(AscApiError::Status(response.status()));
        }

        let body: AppStoreVersionResponse = response.json().await?;
        let attributes = body.data.attributes;

        let version_string = attributes
            .version_string
            .ok_or(AscApiError::MissingField("attributes.versionString"))?;
        let apple_platform = attributes
            .platform
            .ok_or(AscApiError::MissingField("attributes.platform"))?;

        Ok(AppStoreVersionInfo {
            version_string,
            apple_platform,
        })
    }
}

#[derive(Debug, Deserialize)]
struct AppStoreVersionResponse {
    data: AppStoreVersionData,
}

#[derive(Debug, Deserialize)]
struct AppStoreVersionData {
    attributes: AppStoreVersionAttributes,
}

#[derive(Debug, Deserialize)]
struct AppStoreVersionAttributes {
    #[serde(rename = "versionString")]
    version_string: Option<String>,
    platform: Option<String>,
}
