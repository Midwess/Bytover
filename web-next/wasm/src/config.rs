use url::Url;
use wasm_bindgen::JsValue;
use web_sys::window;

pub struct HostInfo {
    pub host: String,
    pub port: u16,
    pub is_with_ssl: bool
}

pub fn get_host_info() -> Result<HostInfo, JsValue> {
    let window = window().ok_or_else(|| JsValue::from_str("No window found"))?;
    let href = window.location().href().map_err(|_| JsValue::from_str("Failed to get href"))?;

    let url = Url::parse(&href).map_err(|e| JsValue::from_str(&format!("Failed to parse URL: {}", e)))?;

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
