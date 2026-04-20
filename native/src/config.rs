pub const GATEWAY_HOST: Option<&str> = option_env!("BYTOVER_PUBLIC_GATEWAY_HOST");
pub const GATEWAY_PORT: Option<&str> = option_env!("BYTOVER_PUBLIC_GATEWAY_PORT");
pub const WITH_SSL: Option<&str> = option_env!("BYTOVER_WITH_SSL");
pub const GATEWAY_HTTP1_HOST: Option<&str> = option_env!("BYTOVER_PUBLIC_HTTP1_GATEWAY_HOST");
pub const GATEWAY_HTTP1_PORT: Option<&str> = option_env!("BYTOVER_PUBLIC_HTTP1_GATEWAY_PORT");
pub const RELAY_ONLY: Option<&str> = option_env!("BYTOVER_RELAY_ONLY");

pub fn get_gateway_grpc_url() -> String {
    let gateway_host = GATEWAY_HOST.unwrap_or("localhost");
    let gateway_port = GATEWAY_PORT.unwrap_or("80");
    if WITH_SSL == Some("1") {
        format!("https://{gateway_host}:{gateway_port}")
    } else {
        format!("http://{gateway_host}:{gateway_port}")
    }
}

pub fn get_signalling_server_ws_url_for_route(route: &str) -> String {
    let gateway_host = GATEWAY_HTTP1_HOST.unwrap_or(GATEWAY_HOST.unwrap_or("localhost"));
    let gateway_port = GATEWAY_HTTP1_PORT
        .map(|it| format!(":{it}"))
        .unwrap_or(GATEWAY_PORT.map(|it| format!(":{it}")).unwrap_or("".to_owned()));
    let route = route.trim_start_matches('/');

    if WITH_SSL == Some("1") {
        format!("wss://{gateway_host}{gateway_port}/{route}")
    } else {
        format!("ws://{gateway_host}{gateway_port}/{route}")
    }
}

pub fn get_signalling_server_http_url_for_route(route: &str) -> String {
    let gateway_host = GATEWAY_HTTP1_HOST.unwrap_or(GATEWAY_HOST.unwrap_or("localhost"));
    let gateway_port = GATEWAY_HTTP1_PORT
        .map(|it| format!(":{it}"))
        .unwrap_or(GATEWAY_PORT.map(|it| format!(":{it}")).unwrap_or("".to_owned()));
    let route = route.trim_start_matches('/');

    if WITH_SSL == Some("1") {
        format!("https://{gateway_host}{gateway_port}/{route}")
    } else {
        format!("http://{gateway_host}{gateway_port}/{route}")
    }
}

pub fn get_updater_url() -> String {
    let gateway_host = GATEWAY_HTTP1_HOST.unwrap_or(GATEWAY_HOST.unwrap_or("api.bytover.com"));
    let gateway_port = GATEWAY_HTTP1_PORT
        .map(|it| format!(":{it}"))
        .unwrap_or(GATEWAY_PORT.map(|it| format!(":{it}")).unwrap_or("".to_owned()));

    if WITH_SSL == Some("1") {
        format!("https://{gateway_host}{gateway_port}/bitbridge/api/v1/update")
    } else {
        format!("http://{gateway_host}{gateway_port}/bitbridge/api/v1/update")
    }
}

pub fn is_relay_only() -> bool {
    RELAY_ONLY == Some("1") || std::env::var("BYTOVER_RELAY_ONLY").ok().map(|v| v == "1").unwrap_or(false)
}

pub fn get_relay_server_override() -> Option<String> {
    std::env::var("BYTOVER_RELAY_SERVER").ok().filter(|s| !s.is_empty())
}

fn env_var(key: &str) -> Option<String> {
    std::env::var(key).ok().map(|value| value.trim().to_string()).filter(|value| !value.is_empty())
}

pub fn get_relay_turn_username() -> String {
    env_var("BYTOVER_RELAY_TURN_USERNAME")
        .or_else(|| env_var("TURN_USERNAME"))
        .unwrap_or_else(|| "relay".to_string())
}

pub fn get_relay_turn_password() -> String {
    env_var("BYTOVER_RELAY_TURN_PASSWORD")
        .or_else(|| env_var("TURN_PASSWORD"))
        .unwrap_or_else(|| "relay-secret".to_string())
}
