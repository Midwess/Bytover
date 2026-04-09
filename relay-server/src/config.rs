use std::env;

fn env_var(key: &str) -> Option<String> {
    env::var(key).ok().map(|value| value.trim().to_string()).filter(|value| !value.is_empty())
}

fn is_local_host(host: &str) -> bool {
    matches!(host, "localhost" | "127.0.0.1" | "host.docker.internal")
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

pub fn get_relay_control_host() -> String {
    env_var("BYTOVER_RELAY_CONTROL_HOST")
        .or_else(|| env_var("SERVICE_HOST"))
        .unwrap_or_else(|| "127.0.0.1".to_string())
}

pub fn get_relay_public_ip() -> Option<String> {
    env_var("BYTOVER_RELAY_PUBLIC_IP").or_else(|| {
        let gateway_host = get_gateway_http_host();
        if is_local_host(&gateway_host) {
            Some("127.0.0.1".to_string())
        } else {
            None
        }
    })
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
