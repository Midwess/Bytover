#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicEndpointConfig {
    pub host: String,
    pub port: u16,
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
    std::env::var(key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn read_port(key: &str) -> Option<u16> {
    std::env::var(key).ok().and_then(|value| value.trim().parse::<u16>().ok())
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
