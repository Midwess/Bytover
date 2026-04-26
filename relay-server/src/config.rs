use std::env;
use std::path::PathBuf;

const DEFAULT_GEOIP_DB_PATH: &str = "/app/assets/GeoLite2-Country.mmdb";

fn env_var(key: &str) -> Option<String> {
    normalize_env(env::var(key).ok())
}

fn normalize_env(raw: Option<String>) -> Option<String> {
    raw.map(|value| value.trim().to_string()).filter(|value| !value.is_empty())
}

pub fn local_region_code() -> Option<String> {
    env_var("BYTOVER_REGION_CODE")
}

pub fn geoip_db_path() -> PathBuf {
    geoip_db_path_from(env_var("BYTOVER_GEOIP_DB_PATH"))
}

fn geoip_db_path_from(value: Option<String>) -> PathBuf {
    value.map(PathBuf::from).unwrap_or_else(|| PathBuf::from(DEFAULT_GEOIP_DB_PATH))
}

pub fn with_ssl() -> bool {
    env_var("BYTOVER_WITH_SSL").as_deref() == Some("1")
}

pub fn get_gateway_grpc_host() -> String {
    env_var("BYTOVER_PUBLIC_GATEWAY_HOST").unwrap_or_else(|| "localhost".to_string())
}

pub fn get_gateway_grpc_port() -> String {
    env_var("BYTOVER_PUBLIC_GATEWAY_PORT").unwrap_or_else(|| "8000".to_string())
}

pub fn get_gateway_grpc_url() -> String {
    let host = get_gateway_grpc_host();
    let port = get_gateway_grpc_port();

    if with_ssl() {
        format!("https://{host}:{port}")
    } else {
        format!("http://{host}:{port}")
    }
}

pub fn get_gateway_http_host() -> String {
    env_var("BYTOVER_PUBLIC_HTTP1_GATEWAY_HOST").unwrap_or_else(get_gateway_grpc_host)
}

pub fn get_gateway_http_port() -> String {
    env_var("BYTOVER_PUBLIC_HTTP1_GATEWAY_PORT").unwrap_or_else(get_gateway_grpc_port)
}

pub fn get_signalling_registration_url(route: &str) -> String {
    let host = get_gateway_http_host();
    let port = get_gateway_http_port();
    let route = route.trim_start_matches('/');

    if with_ssl() {
        format!("https://{host}:{port}/{route}/register-relay")
    } else {
        format!("http://{host}:{port}/{route}/register-relay")
    }
}

#[cfg(test)]
mod tests {
    use super::{geoip_db_path_from, normalize_env, DEFAULT_GEOIP_DB_PATH};
    use std::path::PathBuf;

    #[test]
    fn normalize_env_trims_whitespace_and_drops_empty() {
        assert_eq!(normalize_env(Some("  asia  ".to_string())), Some("asia".to_string()));
        assert_eq!(normalize_env(Some("".to_string())), None);
        assert_eq!(normalize_env(Some("   ".to_string())), None);
        assert_eq!(normalize_env(None), None);
    }

    #[test]
    fn geoip_db_path_defaults_when_unset() {
        let path = geoip_db_path_from(None);
        assert_eq!(path, PathBuf::from(DEFAULT_GEOIP_DB_PATH));
    }

    #[test]
    fn geoip_db_path_uses_override() {
        let path = geoip_db_path_from(Some("/tmp/custom.mmdb".to_string()));
        assert_eq!(path, PathBuf::from("/tmp/custom.mmdb"));
    }
}
