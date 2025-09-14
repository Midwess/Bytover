pub const GATEWAY_HOST: &str = env!("DEVLOG_PUBLIC_GATEWAY_HOST");
pub const GATEWAY_PORT: &str = env!("DEVLOG_PUBLIC_GATEWAY_PORT");
pub const DEVLOG_WITH_SSL: &str = env!("DEVLOG_WITH_SSL");

pub fn get_gateway_grpc_url() -> String {
    if DEVLOG_WITH_SSL == "1" {
        format!("https://{GATEWAY_HOST}:{GATEWAY_PORT}")
    } else {
        format!("http://{GATEWAY_HOST}:{GATEWAY_PORT}")
    }
}

pub fn get_signalling_server_ws_url() -> String {
    if DEVLOG_WITH_SSL == "1" {
        format!("wss://{GATEWAY_HOST}:{GATEWAY_PORT}/rpc-signalling")
    } else {
        format!("ws://{GATEWAY_HOST}:{GATEWAY_PORT}/rpc-signalling")
    }
}

pub fn get_locator_http_url() -> String {
    if DEVLOG_WITH_SSL == "1" {
        format!("https://{GATEWAY_HOST}:{GATEWAY_PORT}/locator")
    } else {
        format!("http://{GATEWAY_HOST}:{GATEWAY_PORT}/locator")
    }
}
