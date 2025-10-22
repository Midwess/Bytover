pub const GATEWAY_HOST: Option<&str> = option_env!("DEVLOG_PUBLIC_GATEWAY_HOST");
pub const GATEWAY_PORT: Option<&str> = option_env!("DEVLOG_PUBLIC_GATEWAY_PORT");
pub const DEVLOG_WITH_SSL: Option<&str> = option_env!("DEVLOG_WITH_SSL");

pub fn get_gateway_grpc_url() -> String {
    let gateway_host = GATEWAY_HOST.unwrap_or("localhost");
    let gateway_port = GATEWAY_PORT.unwrap_or("80");
    if DEVLOG_WITH_SSL == Some("1") {
        format!("https://{gateway_host}:{gateway_port}")
    } else {
        format!("http://{gateway_host}:{gateway_port}")
    }
}

pub fn get_signalling_server_ws_url() -> String {
    let gateway_host = GATEWAY_HOST.unwrap_or("localhost");
    let gateway_port = GATEWAY_PORT.unwrap_or("80");

    if DEVLOG_WITH_SSL == Some("1") {
        format!("wss://{gateway_host}:{gateway_port}/rpc-signalling")
    } else {
        format!("ws://{gateway_host}:{gateway_port}/rpc-signalling")
    }
}

pub fn get_locator_http_url() -> String {
    let gateway_host = GATEWAY_HOST.unwrap_or("localhost");
    let gateway_port = GATEWAY_PORT.unwrap_or("80");

    if DEVLOG_WITH_SSL == Some("1") {
        format!("https://{gateway_host}:{gateway_port}/locator")
    } else {
        format!("http://{gateway_host}:{gateway_port}/locator")
    }
}
