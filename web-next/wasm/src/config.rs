use serde::{Deserialize, Serialize};
use url::Url;
use wasm_bindgen::JsValue;
use wasm_bindgen::JsCast;
use web_sys::{window, WorkerGlobalScope};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostInfo {
    pub host: String,
    pub port: u16,
    pub is_with_ssl: bool,
}

pub fn get_host_info() -> Result<HostInfo, JsValue> {
    let href = if let Some(win) = window() {
        win.location().href()?
    } else {
        let global = js_sys::global().unchecked_into::<WorkerGlobalScope>();
        global.location().href()
    };

    let url = Url::parse(&href)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse URL: {}", e)))?;

    let port = match url.port() {
        Some(p) => p,
        None => match url.scheme() {
            "https" | "wss" => 443,
            "http" | "ws" => 80,
            scheme => return Err(JsValue::from_str(&format!("Unsupported scheme {scheme}"))),
        },
    };

    let is_with_ssl = url.scheme() == "https" || url.scheme() == "wss";

    Ok(HostInfo {
        host: url.host_str().unwrap_or_default().to_string(),
        port,
        is_with_ssl,
    })
}

impl HostInfo {
    pub fn get_gateway_grpc_url(&self) -> String {
        if self.is_with_ssl {
            format!("https://{}:{}", self.host, self.port)
        } else {
            format!("http://{}:{}", self.host, self.port)
        }
    }

    pub fn get_signalling_server_ws_url(&self) -> String {
        if self.is_with_ssl {
            format!("wss://{}:{}/rpc-signalling", self.host, self.port)
        } else {
            format!("ws://{}:{}/rpc-signalling", self.host, self.port)
        }
    }
}