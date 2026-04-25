#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicEndpointConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone)]
pub struct AppStoreConfig {
    pub webhook_secret: Option<Vec<u8>>,
    pub force_update_enabled: bool,
    pub default_store_url_darwin: Option<String>,
    pub default_store_url_ios: Option<String>,
    pub connect_api: Option<AppStoreConnectApiCredentials>,
    pub connect_api_base_url: String,
}

#[derive(Debug, Clone)]
pub struct AppStoreConnectApiCredentials {
    pub issuer_id: String,
    pub key_id: String,
    pub private_key_pem: String,
}

impl AppStoreConfig {
    pub fn default_store_url_for(&self, platform: &str) -> Option<&str> {
        match platform {
            "darwin" => self.default_store_url_darwin.as_deref(),
            "ios" => self.default_store_url_ios.as_deref(),
            _ => None,
        }
    }
}

pub fn load_app_store_config() -> AppStoreConfig {
    let webhook_secret = read_string("APP_STORE_CONNECT_WEBHOOK_SECRET").map(|s| s.into_bytes());
    let force_update_enabled = read_bool("APP_STORE_FORCE_UPDATE_ENABLED").unwrap_or(false);
    let default_store_url_darwin = read_string("APP_STORE_DEFAULT_URL_DARWIN");
    let default_store_url_ios = read_string("APP_STORE_DEFAULT_URL_IOS");
    let connect_api = load_connect_api_credentials();
    let connect_api_base_url = read_string("APP_STORE_CONNECT_API_BASE_URL")
        .unwrap_or_else(|| "https://api.appstoreconnect.apple.com".to_string());

    AppStoreConfig {
        webhook_secret,
        force_update_enabled,
        default_store_url_darwin,
        default_store_url_ios,
        connect_api,
        connect_api_base_url,
    }
}

fn load_connect_api_credentials() -> Option<AppStoreConnectApiCredentials> {
    let issuer_id = read_string("APP_STORE_CONNECT_ISSUER_ID")?;
    let key_id = read_string("APP_STORE_CONNECT_KEY_ID")?;
    let private_key_pem = read_private_key()?;

    Some(AppStoreConnectApiCredentials {
        issuer_id,
        key_id,
        private_key_pem,
    })
}

fn read_private_key() -> Option<String> {
    if let Some(inline) = read_string("APP_STORE_CONNECT_PRIVATE_KEY") {
        return Some(inline);
    }
    let path = read_string("APP_STORE_CONNECT_PRIVATE_KEY_PATH")?;
    match std::fs::read_to_string(&path) {
        Ok(pem) => Some(pem),
        Err(err) => {
            log::error!("Failed to read APP_STORE_CONNECT_PRIVATE_KEY_PATH={}: {}", path, err);
            None
        }
    }
}

pub fn resolve_public_endpoint(default_host: &str, default_port: u16) -> PublicEndpointConfig {
    let host = read_string("SERVICE_PUBLIC_HOST").unwrap_or_else(|| default_host.to_string());
    let port = read_port("SERVICE_PUBLIC_PORT").unwrap_or(default_port);

    PublicEndpointConfig { host, port }
}

fn read_string(key: &str) -> Option<String> {
    std::env::var(key).ok().map(|value| value.trim().to_string()).filter(|value| !value.is_empty())
}

fn read_port(key: &str) -> Option<u16> {
    std::env::var(key).ok().and_then(|value| value.trim().parse::<u16>().ok())
}

fn read_bool(key: &str) -> Option<bool> {
    std::env::var(key).ok().map(|value| {
        matches!(value.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on")
    })
}

#[cfg(test)]
mod tests {
    use super::resolve_public_endpoint;

    #[test]
    fn uses_service_public_port_when_present() {
        std::env::set_var("SERVICE_PUBLIC_PORT", "18080");

        let endpoint = resolve_public_endpoint("localhost", 3000);

        assert_eq!(endpoint.port, 18080);

        std::env::remove_var("SERVICE_PUBLIC_PORT");
    }

    #[test]
    fn uses_service_public_host_when_present() {
        std::env::set_var("SERVICE_PUBLIC_HOST", "backend.internal");

        let endpoint = resolve_public_endpoint("localhost", 3000);

        assert_eq!(endpoint.host, "backend.internal");

        std::env::remove_var("SERVICE_PUBLIC_HOST");
    }

    #[test]
    fn falls_back_to_listener_values() {
        let endpoint = resolve_public_endpoint("127.0.0.1", 3000);

        assert_eq!(endpoint.host, "127.0.0.1");
        assert_eq!(endpoint.port, 3000);
    }
}
