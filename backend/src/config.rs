#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicEndpointConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone)]
pub struct AppStoreConfig {
    pub webhook_secret: Option<Vec<u8>>,
    pub webhook_max_skew: std::time::Duration,
    pub force_update_enabled: bool,
    pub default_store_url_darwin: Option<String>,
    pub default_store_url_ios: Option<String>,
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

const DEFAULT_WEBHOOK_MAX_SKEW_SECS: u64 = 300;

pub fn load_app_store_config() -> AppStoreConfig {
    let webhook_secret = read_string("APP_STORE_CONNECT_WEBHOOK_SECRET").map(|s| s.into_bytes());
    let webhook_max_skew_secs = read_u64("WEBHOOK_MAX_SKEW_SECS").unwrap_or(DEFAULT_WEBHOOK_MAX_SKEW_SECS);
    let force_update_enabled = read_bool("APP_STORE_FORCE_UPDATE_ENABLED").unwrap_or(false);
    let default_store_url_darwin = read_string("APP_STORE_DEFAULT_URL_DARWIN");
    let default_store_url_ios = read_string("APP_STORE_DEFAULT_URL_IOS");

    AppStoreConfig {
        webhook_secret,
        webhook_max_skew: std::time::Duration::from_secs(webhook_max_skew_secs),
        force_update_enabled,
        default_store_url_darwin,
        default_store_url_ios,
    }
}

pub fn resolve_public_grpc_endpoint(default_host: &str, default_port: u16) -> PublicEndpointConfig {
    resolve_public_endpoint(default_host, default_port)
}

fn resolve_public_endpoint(default_host: &str, default_port: u16) -> PublicEndpointConfig {
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

fn read_u64(key: &str) -> Option<u64> {
    std::env::var(key).ok().and_then(|value| value.trim().parse::<u64>().ok())
}

fn read_bool(key: &str) -> Option<bool> {
    std::env::var(key).ok().map(|value| {
        matches!(value.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on")
    })
}

#[cfg(test)]
mod tests {
    use super::resolve_public_grpc_endpoint;

    #[test]
    fn uses_service_public_port_when_present() {
        std::env::set_var("SERVICE_PUBLIC_PORT", "18080");

        let endpoint = resolve_public_grpc_endpoint("localhost", 3000);

        assert_eq!(endpoint.port, 18080);

        std::env::remove_var("SERVICE_PUBLIC_PORT");
    }

    #[test]
    fn uses_service_public_host_when_present() {
        std::env::set_var("SERVICE_PUBLIC_HOST", "backend.internal");

        let endpoint = resolve_public_grpc_endpoint("localhost", 3000);

        assert_eq!(endpoint.host, "backend.internal");

        std::env::remove_var("SERVICE_PUBLIC_HOST");
    }

    #[test]
    fn falls_back_to_listener_values() {
        let endpoint = resolve_public_grpc_endpoint("127.0.0.1", 3000);

        assert_eq!(endpoint.host, "127.0.0.1");
        assert_eq!(endpoint.port, 3000);
    }
}
