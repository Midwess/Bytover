use std::env;

fn env_var(key: &str) -> Option<String> {
    env::var(key).ok().map(|value| value.trim().to_string()).filter(|value| !value.is_empty())
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
