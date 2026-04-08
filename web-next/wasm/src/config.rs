use url::Url;
use wasm_bindgen::JsValue;
use web_sys::window;

/// If these are not specified, we will use the host of the current webpage.
pub const GATEWAY_HOST: Option<&str> = option_env!("BYTOVER_PUBLIC_GATEWAY_HOST");
pub const GATEWAY_PORT: Option<&str> = option_env!("BYTOVER_PUBLIC_GATEWAY_PORT");
pub const WITH_SSL: Option<&str> = option_env!("BYTOVER_WITH_SSL");
pub const GATEWAY_HTTP1_HOST: Option<&str> = option_env!("BYTOVER_PUBLIC_HTTP1_GATEWAY_HOST");
pub const GATEWAY_HTTP1_PORT: Option<&str> = option_env!("BYTOVER_PUBLIC_HTTP1_GATEWAY_PORT");
pub const RELAY_ONLY: Option<&str> = option_env!("BYTOVER_RELAY_ONLY");
pub const RELAY_SERVER: Option<&str> = option_env!("BYTOVER_RELAY_SERVER");

pub fn is_relay_only() -> bool {
    RELAY_ONLY == Some("1")
}

pub fn get_relay_server_override() -> Option<String> {
    if let Some(server) = RELAY_SERVER {
        if !server.is_empty() {
            return Some(server.to_string());
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    if let Ok(server) = std::env::var("BYTOVER_RELAY_SERVER") {
        if !server.is_empty() {
            return Some(server);
        }
    }

    None
}

pub struct HostInfo {
    pub host: String,
    pub port: u16,
    pub is_with_ssl: bool
}

pub fn get_host_info(http1: bool) -> Result<HostInfo, JsValue> {
    let window = window().ok_or_else(|| JsValue::from_str("No window found"))?;
    let href = window.location().href().map_err(|_| JsValue::from_str("Failed to get href"))?;

    let env_scheme = match WITH_SSL.unwrap_or("0") == "1" {
        true => "https",
        false => "http"
    };

    let port = if http1 { GATEWAY_HTTP1_PORT } else { GATEWAY_PORT };

    let host = if http1 { GATEWAY_HTTP1_HOST } else { GATEWAY_HOST };

    let env_url = Url::parse(&format!(
        "{env_scheme}://{}{}",
        host.unwrap_or_default(),
        port.map(|it| format!(":{it}")).unwrap_or_default(),
    ))
    .ok();
    let url = env_url.unwrap_or(Url::parse(&href).map_err(|e| JsValue::from_str(&format!("Failed to parse URL: {}", e)))?);

    let port = match url.port_or_known_default() {
        Some(p) => p,
        None => return Err(JsValue::from_str("Could not determine port"))
    };

    let is_with_ssl = url.scheme() == "https";

    Ok(HostInfo {
        host: url.host_str().unwrap_or_default().to_string(),
        port,
        is_with_ssl
    })
}

pub fn get_gateway_grpc_url() -> String {
    let host_info = get_host_info(false).unwrap();
    if host_info.is_with_ssl {
        format!("https://{}:{}", host_info.host, host_info.port)
    } else {
        format!("http://{}:{}", host_info.host, host_info.port)
    }
}

pub fn get_signalling_server_ws_url() -> String {
    get_signalling_server_ws_url_for_route("rpc-signalling")
}

pub fn get_signalling_server_ws_url_for_route(route: &str) -> String {
    let host_info = get_host_info(true).unwrap();
    let route = route.trim_start_matches('/');
    if host_info.is_with_ssl {
        format!("wss://{}:{}/{}", host_info.host, host_info.port, route)
    } else {
        format!("ws://{}:{}/{}", host_info.host, host_info.port, route)
    }
}

pub fn get_signalling_server_http_url() -> String {
    get_signalling_server_http_url_for_route("rpc-signalling")
}

pub fn get_signalling_server_http_url_for_route(route: &str) -> String {
    let host_info = get_host_info(true).unwrap();
    let route = route.trim_start_matches('/');
    if host_info.is_with_ssl {
        format!("https://{}:{}/{}", host_info.host, host_info.port, route)
    } else {
        format!("http://{}:{}/{}", host_info.host, host_info.port, route)
    }
}
