pub const GATEWAY_HOST: Option<&str> = option_env!("BYTOVER_PUBLIC_GATEWAY_HOST");
pub const GATEWAY_PORT: Option<&str> = option_env!("BYTOVER_PUBLIC_GATEWAY_PORT");
pub const WITH_SSL: Option<&str> = option_env!("BYTOVER_WITH_SSL");
pub const LOCATOR_URL: Option<&str> = option_env!("BYTOVER_LOCATOR_URL");
pub const GATEWAY_HTTP1_HOST: Option<&str> = option_env!("BYTOVER_PUBLIC_HTTP1_GATEWAY_HOST");
pub const GATEWAY_HTTP1_PORT: Option<&str> = option_env!("BYTOVER_PUBLIC_HTTP1_GATEWAY_PORT");

pub fn get_gateway_grpc_url() -> String {
    let gateway_host = GATEWAY_HOST.unwrap_or("localhost");
    let gateway_port = GATEWAY_PORT.unwrap_or("80");
    if WITH_SSL == Some("1") {
        format!("https://{gateway_host}:{gateway_port}")
    } else {
        format!("http://{gateway_host}:{gateway_port}")
    }
}

pub fn get_locator_url() -> String {
    LOCATOR_URL.unwrap_or("https://bytover.com/locator").to_string()
}

pub fn get_signalling_server_ws_url() -> String {
    let gateway_host = GATEWAY_HTTP1_HOST.unwrap_or(GATEWAY_HOST.unwrap_or("localhost"));
    let gateway_port = GATEWAY_HTTP1_PORT
        .map(|it| format!(":{it}"))
        .unwrap_or(GATEWAY_PORT.map(|it| format!(":{it}")).unwrap_or("".to_owned()));

    if WITH_SSL == Some("1") {
        format!("wss://{gateway_host}{gateway_port}/rpc-signalling")
    } else {
        format!("ws://{gateway_host}{gateway_port}/rpc-signalling")
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
