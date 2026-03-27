use js_sys::Uint8Array;
use prost::Message as ProstMessage;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

use core_services::wasm::http::HttpClient;
use schema::devlog::rpc_signalling::server::{Message, OfferMessage};

pub struct SignalingClient {
    url: String,
}

impl SignalingClient {
    pub fn new(url: impl Into<String>) -> Self {
        Self { url: url.into() }
    }

    pub async fn send_offer(&self, peer_id: &str, offer_sdp: &str) -> Result<String, SignalingError> {
        let url = format!("{}/offer/{}", self.url.trim_end_matches('/'), peer_id);

        let offer_msg = Message {
            request_id: None,
            offer: Some(OfferMessage { sdp: offer_sdp.to_string() }),
            answer: None,
            error: None,
            ice_config: None,
        };

        let mut encoded = Vec::new();
        prost::Message::encode(&offer_msg, &mut encoded)
            .map_err(|e| SignalingError::Encoding(e.to_string()))?;

        let response = HttpClient::new()
            .method("POST")
            .url(&url)
            .header("Content-Type", "application/octet-stream")
            .body_bytes(encoded)
            .fetch()
            .map_err(|e| SignalingError::Network(format!("{:?}", e)))?
            .send()
            .await
            .map_err(|e| SignalingError::Network(format!("{:?}", e)))?;

        let status = response.status();

        let array_buffer: js_sys::ArrayBuffer = JsFuture::from(
            response.array_buffer().map_err(|e| SignalingError::Network(format!("{:?}", e)))?,
        )
        .await
        .map_err(|e| SignalingError::Network(format!("{:?}", e)))?
        .dyn_into()
        .map_err(|e| SignalingError::Network(format!("{:?}", e)))?;

        let bytes = Uint8Array::new(&array_buffer).to_vec();

        if status != 200 {
            return Err(SignalingError::Server(
                String::from_utf8_lossy(&bytes).to_string(),
            ));
        }

        let response_msg = Message::decode(&bytes[..])
            .map_err(|e| SignalingError::Decoding(format!("{:?}", e)))?;

        response_msg
            .answer
            .ok_or(SignalingError::InvalidResponse)
            .map(|a| a.sdp)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SignalingError {
    #[error("Network error: {0}")]
    Network(String),

    #[error("Encoding error: {0}")]
    Encoding(String),

    #[error("Decoding error: {0}")]
    Decoding(String),

    #[error("Server error: {0}")]
    Server(String),

    #[error("Timeout")]
    Timeout,

    #[error("Invalid response")]
    InvalidResponse,
}
