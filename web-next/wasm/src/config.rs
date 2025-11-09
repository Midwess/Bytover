use url::Url;
use wasm_bindgen::JsValue;
use web_sys::window;

/// If these are not specified, we will use the host of the current webpage.
pub const GATEWAY_HOST: Option<&str> = option_env!("DEVLOG_PUBLIC_GATEWAY_HOST");
pub const GATEWAY_PORT: Option<&str> = option_env!("DEVLOG_PUBLIC_GATEWAY_PORT");
pub const WITH_SSL: Option<&str> = option_env!("BITBRIDGE_WITH_SSL");
pub const LOCATOR_URL: Option<&str> = option_env!("BITBRIDGE_LOCATOR_URL");

pub struct HostInfo {
    pub host: String,
    pub port: u16,
    pub is_with_ssl: bool
}

pub fn get_locator_url() -> String {
    LOCATOR_URL.unwrap_or("https://devlog.studio/locator").to_string()
}

pub fn get_host_info() -> Result<HostInfo, JsValue> {
    let window = window().ok_or_else(|| JsValue::from_str("No window found"))?;
    let href = window.location().href().map_err(|_| JsValue::from_str("Failed to get href"))?;

    let env_scheme = match WITH_SSL.unwrap_or("0") == "1" {
        true => "https",
        false => "http"
    };

    let env_url = Url::parse(&format!(
        "{env_scheme}://{}:{}",
        GATEWAY_HOST.unwrap_or_default(),
        GATEWAY_PORT.unwrap_or_default()
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
    let host_info = get_host_info().unwrap();
    if host_info.is_with_ssl {
        format!("https://{}:{}", host_info.host, host_info.port)
    } else {
        format!("http://{}:{}", host_info.host, host_info.port)
    }
}

pub fn get_signalling_server_ws_url() -> String {
    let host_info = get_host_info().unwrap();
    if host_info.is_with_ssl {
        format!("wss://{}:{}/rpc-signalling", host_info.host, host_info.port)
    } else {
        format!("ws://{}:{}/rpc-signalling", host_info.host, host_info.port)
    }
}
